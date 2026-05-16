use async_trait::async_trait;
use std::time::{Duration, Instant};

use arena::healthcheck::ReadinessCheck;
use futures_timer::Delay;

pub(super) struct LocalstackHealthReadinessCheck {
    readiness_scope: Vec<String>,
}

impl LocalstackHealthReadinessCheck {
    pub fn new(readiness_scope: Vec<String>) -> Self {
        Self { readiness_scope }
    }
}

const ACCEPTABLE_STATES: &[&str] = &["running", "available", "disabled"];

#[async_trait]
impl ReadinessCheck for LocalstackHealthReadinessCheck {
    async fn is_ready(
        &self,
        identifier: &str,
        endpoint: &str,
        timeout_ms: u64,
    ) -> Result<(), String> {
        let timeout = Duration::from_millis(timeout_ms);
        let poll_every = Duration::from_millis(100);
        let health_url = format!("{}/_localstack/health", endpoint.trim_end_matches('/'));

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .map_err(|e| format!("build reqwest client: {e}"))?;

        let start = Instant::now();
        let mut last_err = String::from("no attempts made");

        while start.elapsed() < timeout {
            match poll_once(&client, &health_url, &self.readiness_scope).await {
                Ok(()) => {
                    tracing::debug!(
                        identifier = %identifier,
                        "localstack health endpoint reports scoped services ready"
                    );
                    return Ok(());
                }
                Err(err) => {
                    tracing::debug!(
                        identifier = %identifier,
                        error = %err,
                        "readiness probe failed (will retry)"
                    );
                    last_err = err;
                    Delay::new(poll_every).await;
                }
            }
        }

        Err(format!(
            "localstack healthcheck timed out after {timeout_ms}ms (last error: {last_err})"
        ))
    }
}

async fn poll_once(
    client: &reqwest::Client,
    url: &str,
    readiness_scope: &[String],
) -> Result<(), String> {
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("health request failed: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("health endpoint returned {}", resp.status()));
    }

    let body: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("health body decode failed: {e}"))?;

    let services = body
        .get("services")
        .and_then(|v| v.as_object())
        .ok_or_else(|| "health body missing 'services' object".to_string())?;

    if readiness_scope.is_empty() {
        for (name, state) in services {
            let state_str = state.as_str().unwrap_or("");
            if !ACCEPTABLE_STATES.contains(&state_str) {
                return Err(format!("service {name} is in state '{state_str}'"));
            }
        }
    } else {
        for name in readiness_scope {
            let state = services
                .get(name)
                .ok_or_else(|| format!("health body missing service '{name}'"))?;
            let state_str = state.as_str().unwrap_or("");
            if !ACCEPTABLE_STATES.contains(&state_str) {
                return Err(format!("service {name} is in state '{state_str}'"));
            }
        }
    }

    Ok(())
}
