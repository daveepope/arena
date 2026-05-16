use arena::healthcheck::ReadinessCheck;
use async_trait::async_trait;
use std::time::Duration;

pub struct HttpReadinessCheck;

impl HttpReadinessCheck {
    pub fn new() -> Self {
        Self
    }
}

impl Default for HttpReadinessCheck {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ReadinessCheck for HttpReadinessCheck {
    async fn is_ready(
        &self,
        identifier: &str,
        target: &str,
        timeout_ms: u64,
    ) -> Result<(), String> {
        let start = std::time::Instant::now();
        let timeout = Duration::from_millis(timeout_ms);
        let poll_interval = Duration::from_millis(100);

        while start.elapsed() < timeout {
            match reqwest::get(target).await {
                Ok(_) => {
                    tracing::debug!(
                        component = %identifier,
                        target = %target,
                        phase = "http_readiness",
                        "http readiness ok",
                    );
                    return Ok(());
                }
                Err(e) => {
                    tracing::trace!(
                        component = %identifier,
                        target = %target,
                        error = %e,
                        phase = "http_readiness_retry",
                        "http readiness probe failed",
                    );
                }
            }

            tokio::time::sleep(poll_interval).await;
        }

        Err(format!(
            "http readiness timed out identifier={identifier} deadline={timeout:?}",
        ))
    }
}
