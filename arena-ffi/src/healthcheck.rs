use arena::healthcheck::ReadinessCheck;
use async_trait::async_trait;
use serde::Deserialize;
use std::time::Duration;

fn default_readiness_timeout_ms() -> u64 {
    10_000
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum ReadinessCheckConfig {
    Http {
        target: String,
        #[serde(default = "default_readiness_timeout_ms")]
        timeout_ms: u64,
    },
    Tcp {
        target: String,
        #[serde(default = "default_readiness_timeout_ms")]
        timeout_ms: u64,
    },
}

pub(crate) struct HttpReadinessCheck;

impl HttpReadinessCheck {
    pub fn new() -> Self {
        Self
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
                    tracing::info!(
                        identifier = %identifier,
                        target = %target,
                        "http readiness passed"
                    );
                    return Ok(());
                }
                Err(e) => {
                    tracing::trace!(
                        identifier = %identifier,
                        target = %target,
                        error = %e,
                        "http readiness poll failed"
                    );
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

pub(crate) struct TcpReadinessCheck;

impl TcpReadinessCheck {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ReadinessCheck for TcpReadinessCheck {
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
            match tokio::net::TcpStream::connect(target).await {
                Ok(_) => {
                    tracing::info!(
                        identifier = %identifier,
                        target = %target,
                        "tcp readiness passed"
                    );
                    return Ok(());
                }
                Err(e) => {
                    tracing::trace!(
                        identifier = %identifier,
                        target = %target,
                        error = %e,
                        "tcp readiness poll failed"
                    );
                }
            }
            tokio::time::sleep(poll_interval).await;
        }

        Err(format!(
            "[{}] TCP healthcheck timed out after {:?}",
            identifier, timeout
        ))
    }
}

#[cfg(test)]
#[path = "healthcheck_tests.rs"]
mod healthcheck_tests;
