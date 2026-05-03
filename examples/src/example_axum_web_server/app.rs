use std::net::SocketAddr;
use std::sync::Arc;

use arena_mssql::Client as MssqlClient;
use rdkafka::producer::BaseProducer;
use tokio::sync::Mutex;
use tokio_postgres::Client as PgClient;

use super::oauth::JwksValidator;
use super::router::build_router;
use super::state::{build_http_client_trusting_oauth_ca, AppState};

pub struct ExampleAxumWebApp {
    pg: Arc<PgClient>,
    kafka: Arc<BaseProducer>,
    kafka_topic: Arc<str>,
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
        use rdkafka::config::ClientConfig;
        use tokio_postgres::NoTls;

        let (pg, connection) = tokio_postgres::connect(postgres_connection_string, NoTls)
            .await
            .expect("connect to postgres");

        tokio::spawn(async move {
            if let Err(e) = connection.await {
                log::error!("postgres connection error: {e}");
            }
        });

        let kafka: BaseProducer = ClientConfig::new()
            .set("bootstrap.servers", kafka_bootstrap)
            .set("message.timeout.ms", "5000")
            .create()
            .expect("create kafka producer");

        let mssql = arena_mssql::connect(mssql_connection_string)
            .await
            .expect("connect to mssql validation db");

        let http_client = build_http_client_trusting_oauth_ca(oauth_tls_ca_pem);

        Self {
            pg: Arc::new(pg),
            kafka: Arc::new(kafka),
            kafka_topic: Arc::from(kafka_topic),
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
            JwksValidator::from_issuer(&self.http_client, &self.oauth_issuer_url)
                .await
                .map_err(|e| format!("load JWKS from issuer {}: {e}", self.oauth_issuer_url))?,
        );

        let state = AppState {
            pg: self.pg,
            kafka: self.kafka,
            kafka_topic: self.kafka_topic,
            http_client: self.http_client,
            calibration_url: self.calibration_url,
            mssql: self.mssql,
            jwt,
        };

        let app = build_router(state);

        let addr: SocketAddr = format!("0.0.0.0:{}", port).parse()?;
        let listener = tokio::net::TcpListener::bind(addr).await?;
        log::info!("listening on http://{addr}");

        axum::serve(listener, app)
            .with_graceful_shutdown(async {
                let _ = shutdown_signal.await;
            })
            .await?;

        Ok(())
    }
}
