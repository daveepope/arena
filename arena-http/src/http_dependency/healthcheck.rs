use arena::healthcheck::ReadinessCheck;
use async_trait::async_trait;
use std::time::{Duration, Instant};

use crate::admin_client::admin_api_client;

#[derive(Default)]
pub(super) struct DefaultHttpReadinessCheck {
    trusted_tls_certificate_pem: Option<String>,
}

impl DefaultHttpReadinessCheck {
    pub(super) fn new(trusted_tls_certificate_pem: Option<String>) -> Self {
        Self {
            trusted_tls_certificate_pem,
        }
    }
}

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
        let client = admin_api_client(admin_url, self.trusted_tls_certificate_pem.as_deref())
            .expect("HTTP admin API client");
        let mappings_url = format!("{admin_url}/mappings");

        loop {
            if start.elapsed() >= timeout {
                return Err(format!(
                    "readiness timed out identifier={identifier} deadline={timeout:?}"
                ));
            }

            match client.get(&mappings_url).send().await {
                Ok(resp) if resp.status().is_success() => return Ok(()),
                Ok(resp) => {
                    tracing::debug!(
                        identifier = %identifier,
                        status = %resp.status(),
                        "admin mappings endpoint not successful yet"
                    );
                }
                Err(err) => {
                    tracing::debug!(
                        identifier = %identifier,
                        error = %err,
                        "readiness probe failed (will retry)"
                    );
                }
            }

            futures_timer::Delay::new(poll_every).await;
        }
    }
}
