use std::future::Future;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use arena_kafka::kafka_dependency::client::{connect_client, partition_client_for};
use arena_mssql::Client as MssqlClient;
use rskafka::client::partition::PartitionClient;
use tokio::sync::Mutex;
use tokio_postgres::Client as PgClient;

use super::oauth::JwksValidator;
use super::router::build_router;
use super::state::{build_http_client_trusting_oauth_ca, AppState};

const STARTUP_RETRY_ATTEMPTS: u32 = 5;
const STARTUP_RETRY_TIMEOUT: Duration = Duration::from_secs(3);
const STARTUP_RETRY_BACKOFF_BASE: Duration = Duration::from_millis(250);

async fn retry_startup<T, E, F, Fut>(mut attempt: F) -> Result<T, String>
where
    E: std::fmt::Display,
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>,
{
    let mut last_err = String::new();

    for i in 0..STARTUP_RETRY_ATTEMPTS {
        match tokio::time::timeout(STARTUP_RETRY_TIMEOUT, attempt()).await {
            Ok(Ok(value)) => return Ok(value),
            Ok(Err(e)) => last_err = e.to_string(),
            Err(_) => last_err = format!("exceeded {STARTUP_RETRY_TIMEOUT:?}"),
        }

        if i + 1 < STARTUP_RETRY_ATTEMPTS {
            tokio::time::sleep(STARTUP_RETRY_BACKOFF_BASE * 2u32.pow(i)).await;
        }
    }

    Err(format!(
        "failed after {STARTUP_RETRY_ATTEMPTS} attempts: {last_err}"
    ))
}

pub struct ExampleAxumWebApp {
    pg: Arc<PgClient>,
    kafka: Arc<PartitionClient>,
    http_client: reqwest::Client,
    calibration_url: Arc<str>,
    mssql: Arc<Mutex<MssqlClient>>,
    oauth_issuer_url: String,
}

impl ExampleAxumWebApp {
    pub async fn new(
        postgres_connection_string: &str,
        kafka_bootstrap: &str,
        kafka_topic: &str,
        calibration_url: &str,
        mssql_connection_string: &str,
        oauth_issuer_url: &str,
        oauth_tls_ca_pem: &str,
    ) -> Self {
        use tokio_postgres::NoTls;

        let (pg, connection) = retry_startup(|| tokio_postgres::connect(postgres_connection_string, NoTls))
            .await
            .expect("connect to postgres");

        tokio::spawn(async move {
            if let Err(e) = connection.await {
                tracing::error!(error = %e, phase = "postgres_conn_task", "background postgres connection dropped");
            }
        });

        let kafka_client = connect_client(kafka_bootstrap)
            .await
            .expect("create kafka client");
        let kafka = partition_client_for(&kafka_client, kafka_topic)
            .await
            .expect("create kafka partition client");

        let mssql = arena_mssql::connect(mssql_connection_string)
            .await
            .expect("connect to mssql validation db");

        let http_client = build_http_client_trusting_oauth_ca(oauth_tls_ca_pem);

        Self {
            pg: Arc::new(pg),
            kafka: Arc::new(kafka),
            http_client,
            calibration_url: Arc::from(calibration_url),
            mssql: Arc::new(Mutex::new(mssql)),
            oauth_issuer_url: oauth_issuer_url.to_string(),
        }
    }

    pub async fn serve(
        self,
        port: u16,
        shutdown_signal: tokio::sync::oneshot::Receiver<()>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let jwt = Arc::new(
            retry_startup(|| JwksValidator::from_issuer(&self.http_client, &self.oauth_issuer_url))
                .await
                .map_err(|e| format!("load JWKS from issuer {}: {e}", self.oauth_issuer_url))?,
        );

        let required_access_token_scopes: Arc<Vec<String>> =
            std::env::var("OAUTH_REQUIRED_ACCESS_TOKEN_SCOPES")
                .ok()
                .map(|raw| {
                    raw.split_whitespace()
                        .map(String::from)
                        .filter(|s| !s.is_empty())
                        .collect::<Vec<String>>()
                })
                .unwrap_or_default()
                .into();

        let state = AppState {
            pg: self.pg,
            kafka: self.kafka,
            http_client: self.http_client,
            calibration_url: self.calibration_url,
            mssql: self.mssql,
            jwt,
            required_access_token_scopes,
        };

        let app = build_router(state);

        let addr: SocketAddr = format!("0.0.0.0:{}", port).parse()?;
        let listener = tokio::net::TcpListener::bind(addr).await?;
        tracing::info!(listen_addr = %addr, phase = "http_listen_begin", "listening");

        axum::serve(listener, app)
            .with_graceful_shutdown(async {
                let _ = shutdown_signal.await;
            })
            .await?;

        Ok(())
    }
}
