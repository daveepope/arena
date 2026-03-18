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
                    log::debug!("[{}] HTTP healthcheck passed", identifier);
                    return Ok(());
                }
                Err(e) => {
                    log::trace!("[{}] HTTP healthcheck failed: {}", identifier, e);
                }
            }

            tokio::time::sleep(poll_interval).await;
        }

        Err(format!(
            "[{}] HTTP healthcheck timed out after {:?}",
            identifier, timeout
        ))
    }
}
