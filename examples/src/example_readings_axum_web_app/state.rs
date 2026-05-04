use std::sync::Arc;

use arena_mssql::Client as MssqlClient;
use rdkafka::producer::BaseProducer;
use tokio::sync::Mutex;
use tokio_postgres::Client as PgClient;

use super::oauth::JwksValidator;

#[derive(Clone)]
pub struct AppState {
    pub pg: Arc<PgClient>,
    pub kafka: Arc<BaseProducer>,
    pub kafka_topic: Arc<str>,
    pub http_client: reqwest::Client,
    pub calibration_url: Arc<str>,
    pub mssql: Arc<Mutex<MssqlClient>>,
    pub jwt: Arc<JwksValidator>,
    pub required_access_token_scopes: Arc<Vec<String>>,
}

pub fn build_http_client_trusting_oauth_ca(oauth_tls_ca_pem: &str) -> reqwest::Client {
    let _ = reqwest::Certificate::from_pem(oauth_tls_ca_pem.as_bytes())
        .expect("OAUTH_TLS_CA_PEM must be valid PEM");
    reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .no_proxy()
        .build()
        .expect("build reqwest client for OAuth + outbound HTTP")
}
