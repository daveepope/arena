use arena::healthcheck::ReadinessCheck;
use async_trait::async_trait;
use std::time::{Duration, Instant};
use tiberius::Config;

pub const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(2);

#[async_trait]
pub trait MssqlHealthcheckOps: Send + Sync {
    async fn ping(&self, conn_str: &str) -> Result<(), String>;
}

pub(super) struct MssqlClientHealthcheckOps {
    connect_timeout: Option<Duration>,
}

impl MssqlClientHealthcheckOps {
    pub(super) fn new(connect_timeout: Option<Duration>) -> Self {
        Self { connect_timeout }
    }
}

#[async_trait]
impl MssqlHealthcheckOps for MssqlClientHealthcheckOps {
    async fn ping(&self, conn_str: &str) -> Result<(), String> {
        let config = Config::from_ado_string(conn_str)
            .map_err(|e| format!("parse ADO connection string: {e}"))?;

        let mut client =
            super::mssql_container_impl::connect_with_config(config, self.connect_timeout)
                .await
                .map_err(|err| format!("mssql connect failed: {err}"))?;

        client
            .simple_query("SELECT 1")
            .await
            .map(|_res| ())
            .map_err(|err| format!("mssql ping query failed: {err}"))
    }
}

async fn run_with_retry(
    ops: &(impl MssqlHealthcheckOps + ?Sized),
    identifier: &str,
    connection_string: &str,
    timeout_ms: u64,
) -> Result<(), String> {
    let timeout_duration = Duration::from_millis(timeout_ms);

    #[cfg(test)]
    let poll_every = Duration::from_millis(1);
    #[cfg(not(test))]
    let poll_every = Duration::from_millis(250);

    let start = Instant::now();
    loop {
        if start.elapsed() >= timeout_duration {
            return Err(format!(
                "[MssqlDependency-{}] mssql did not become ready within {:?}. connection_string={:?}",
                identifier, timeout_duration, connection_string
            ));
        }

        match ops.ping(connection_string).await {
            Ok(()) => return Ok(()),
            Err(err) => tracing::debug!(
                subsystem = "mssql",
                error = %err,
                "readiness probe failed (will retry)"
            ),
        };

        tokio::time::sleep(poll_every).await;
    }
}

pub struct DefaultMssqlReadinessCheck {
    connect_timeout: Option<Duration>,
}

impl DefaultMssqlReadinessCheck {
    pub fn new() -> Self {
        Self {
            connect_timeout: Some(DEFAULT_CONNECT_TIMEOUT),
        }
    }

    pub fn with_connect_timeout(mut self, connect_timeout: Option<Duration>) -> Self {
        self.connect_timeout = connect_timeout;
        self
    }

    pub fn connect_timeout(&self) -> Option<Duration> {
        self.connect_timeout
    }
}

impl Default for DefaultMssqlReadinessCheck {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ReadinessCheck for DefaultMssqlReadinessCheck {
    async fn is_ready(
        &self,
        identifier: &str,
        connection_string: &str,
        timeout_ms: u64,
    ) -> Result<(), String> {
        let ops = MssqlClientHealthcheckOps::new(self.connect_timeout);
        run_with_retry(&ops, identifier, connection_string, timeout_ms).await
    }
}
