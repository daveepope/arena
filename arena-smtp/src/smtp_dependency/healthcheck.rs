use arena::healthcheck::ReadinessCheck;
use async_trait::async_trait;
use futures_timer::Delay;
use std::time::{Duration, Instant};
use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;

pub(super) struct DefaultSmtpReadinessCheck;

const PROBE_TIMEOUT: Duration = Duration::from_secs(2);

#[async_trait]
impl ReadinessCheck for DefaultSmtpReadinessCheck {
    async fn is_ready(
        &self,
        identifier: &str,
        smtp_address: &str,
        timeout_ms: u64,
    ) -> Result<(), String> {
        let timeout = Duration::from_millis(timeout_ms);
        let poll_every = Duration::from_millis(100);
        let start = Instant::now();

        loop {
            match probe_once(smtp_address).await {
                Ok(()) => return Ok(()),
                Err(err) => {
                    if start.elapsed() >= timeout {
                        return Err(format!(
                            "[Smtp-{identifier}] readiness probe against {smtp_address} \
                             did not succeed within {timeout:?}: {err}"
                        ));
                    }

                    tracing::debug!(
                        subsystem = "smtp",
                        error = %err,
                        "readiness probe failed (will retry)"
                    );
                    Delay::new(poll_every).await;
                }
            }
        }
    }
}

async fn probe_once(smtp_address: &str) -> Result<(), String> {
    let mut stream = tokio::time::timeout(PROBE_TIMEOUT, TcpStream::connect(smtp_address))
        .await
        .map_err(|_| "connect timed out".to_string())?
        .map_err(|err| err.to_string())?;

    let mut buffer = [0u8; 3];
    tokio::time::timeout(PROBE_TIMEOUT, stream.read_exact(&mut buffer))
        .await
        .map_err(|_| "banner read timed out".to_string())?
        .map_err(|err| err.to_string())?;

    if &buffer == b"220" {
        Ok(())
    } else {
        Err(format!(
            "unexpected smtp banner prefix: {}",
            String::from_utf8_lossy(&buffer)
        ))
    }
}
