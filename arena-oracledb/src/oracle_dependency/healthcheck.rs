use arena::healthcheck::ReadinessCheck;
use async_trait::async_trait;
use futures_timer::Delay;
use std::time::{Duration, Instant};
use tokio::net::TcpStream;

const PROBE_TIMEOUT: Duration = Duration::from_secs(2);

pub struct DefaultOracleReadinessCheck;

impl DefaultOracleReadinessCheck {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DefaultOracleReadinessCheck {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ReadinessCheck for DefaultOracleReadinessCheck {
    async fn is_ready(
        &self,
        identifier: &str,
        target: &str,
        timeout_ms: u64,
    ) -> Result<(), String> {
        let timeout = Duration::from_millis(timeout_ms);
        let poll_every = Duration::from_millis(100);
        let start = Instant::now();

        loop {
            match probe_once(target).await {
                Ok(()) => return Ok(()),
                Err(err) => {
                    if start.elapsed() >= timeout {
                        return Err(format!(
                            "[Oracle-{identifier}] readiness probe against {target} \
                             did not succeed within {timeout:?}: {err}"
                        ));
                    }

                    tracing::debug!(
                        subsystem = "oracle",
                        error = %err,
                        "readiness probe failed (will retry)"
                    );
                    Delay::new(poll_every).await;
                }
            }
        }
    }
}

async fn probe_once(target: &str) -> Result<(), String> {
    tokio::time::timeout(PROBE_TIMEOUT, TcpStream::connect(target))
        .await
        .map_err(|_| "connect timed out".to_string())?
        .map_err(|err| err.to_string())?;
    Ok(())
}
