use arena::healthcheck::ReadinessCheck;
use async_trait::async_trait;
use std::time::{Duration, Instant};

pub(super) struct DefaultHttpReadinessCheck;

#[async_trait]
impl ReadinessCheck for DefaultHttpReadinessCheck {
    async fn is_ready(
        &self,
        identifier: &str,
        admin_url: &str,
        timeout_ms: u64,
    ) -> Result<(), String> {
        let timeout = Duration::from_millis(timeout_ms);
        let poll_every = Duration::from_millis(100);
        let start = Instant::now();
        let client = reqwest::Client::new();
        let mappings_url = format!("{admin_url}/mappings");

        loop {
            if start.elapsed() >= timeout {
                return Err(format!(
                    "[Http-{identifier}] did not become ready within {timeout:?}"
                ));
            }

            match client.get(&mappings_url).send().await {
                Ok(resp) if resp.status().is_success() => return Ok(()),
                Ok(resp) => {
                    log::debug!(
                        "[Http-{identifier}] healthcheck got status {}, retrying",
                        resp.status()
                    );
                }
                Err(err) => {
                    log::debug!(
                        "[Http-{identifier}] healthcheck failed (will retry): {err}"
                    );
                }
            }

            futures_timer::Delay::new(poll_every).await;
        }
    }
}
