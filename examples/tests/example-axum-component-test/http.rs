use arena_examples::example_readings_axum_web_app::state::build_http_client_trusting_oauth_ca;
use rdkafka::config::ClientConfig;
use rdkafka::consumer::{BaseConsumer, Consumer};
use rdkafka::message::Message;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use std::time::{Duration, Instant};
use tokio::sync::OnceCell;

use crate::arena::{oauth_issuer, oauth_server_tls_cert_pem};

static OAUTH_HTTP_CLIENT: OnceLock<Client> = OnceLock::new();
static ACCESS_TOKEN: OnceCell<String> = OnceCell::const_new();

fn oauth_http_client() -> &'static Client {
    OAUTH_HTTP_CLIENT.get_or_init(|| build_http_client_trusting_oauth_ca(oauth_server_tls_cert_pem()))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Reading {
    pub id: i32,
    pub user_name: String,
    pub value: i32,
    pub comment: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateReadingResponse {
    pub valid: bool,
    #[serde(default)]
    pub id: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct CreateReadingRequest {
    pub user_name: String,
    pub value: i32,
    pub comment: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ReadingCreatedEvent {
    pub id: i64,
    pub user_name: String,
    pub value: i32,
    pub comment: Option<String>,
}

pub async fn fetch_example_access_token() -> String {
    ACCESS_TOKEN
        .get_or_init(|| async {
            arena_examples::oauth_client_credentials::fetch_client_credentials_access_token(
                oauth_http_client(),
                oauth_issuer(),
                Some("openid profile readings"),
            )
            .await
            .expect("fetch client_credentials access token")
        })
        .await
        .clone()
}

pub async fn get_readings(port: u16) -> Vec<Reading> {
    let token = fetch_example_access_token().await;
    let url = format!("http://127.0.0.1:{}/readings", port);
    let response = oauth_http_client()
        .get(&url)
        .bearer_auth(token)
        .send()
        .await
        .expect("GET /readings failed to send");

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        panic!("GET /readings failed (HTTP {status}): {body}");
    }

    response
        .json::<Vec<Reading>>()
        .await
        .expect("GET /readings returned invalid JSON")
}

pub fn consume_reading_created_event(
    bootstrap: String,
    topic: String,
    id_rx: std::sync::mpsc::Receiver<i64>,
    timeout: Duration,
) -> Result<ReadingCreatedEvent, String> {
    let consumer: BaseConsumer = ClientConfig::new()
        .set("bootstrap.servers", &bootstrap)
        .set("group.id", format!("component-test-{}", std::process::id()))
        .set("auto.offset.reset", "earliest")
        .create()
        .map_err(|e| format!("create kafka consumer failed: {e}"))?;

    consumer
        .subscribe(&[&topic])
        .map_err(|e| format!("kafka subscribe failed: {e}"))?;

    let expected_id = id_rx.recv().map_err(|_| "id channel closed")?;

    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        match consumer.poll(Duration::from_millis(100)) {
            None => continue,
            Some(Err(e)) => return Err(format!("kafka consume error: {e}")),
            Some(Ok(msg)) => {
                let payload = match msg.payload() {
                    Some(bytes) => bytes,
                    None => continue,
                };
                let event: ReadingCreatedEvent = serde_json::from_slice(payload)
                    .map_err(|e| format!("parse ReadingCreatedEvent failed: {e}"))?;
                if event.id == expected_id {
                    return Ok(event);
                }
            }
        }
    }
    Err("did not receive expected ReadingCreatedEvent before timeout".to_string())
}

pub async fn post_reading_raw(
    port: u16,
    user_name: &str,
    value: i32,
    comment: Option<String>,
) -> u16 {
    let token = fetch_example_access_token().await;
    let url = format!("http://127.0.0.1:{}/readings", port);
    oauth_http_client()
        .post(&url)
        .bearer_auth(token)
        .json(&CreateReadingRequest {
            user_name: user_name.to_string(),
            value,
            comment,
        })
        .send()
        .await
        .expect("POST /readings failed to send")
        .status()
        .as_u16()
}

pub async fn create_reading(
    port: u16,
    user_name: &str,
    value: i32,
    comment: Option<String>,
) -> i32 {
    let token = fetch_example_access_token().await;
    let url = format!("http://127.0.0.1:{}/readings", port);
    let request = CreateReadingRequest {
        user_name: user_name.to_string(),
        value,
        comment,
    };

    let response = oauth_http_client()
        .post(&url)
        .bearer_auth(token)
        .json(&request)
        .send()
        .await
        .expect("POST /readings failed to send");

    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    if !status.is_success() {
        panic!("POST /readings failed (HTTP {status}): {body}");
    }

    let create_response: CreateReadingResponse = serde_json::from_str(&body)
        .unwrap_or_else(|e| panic!("POST /readings returned invalid JSON: {e}; body: {body}"));

    assert!(
        create_response.valid,
        "expected calibration valid=true in response body: {body}"
    );
    create_response
        .id
        .expect("expected id when calibration accepted reading")
        .try_into()
        .expect("reading id fits i32")
}
