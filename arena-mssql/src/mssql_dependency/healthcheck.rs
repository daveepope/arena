use arena::healthcheck::ReadinessCheck;
use async_trait::async_trait;
use std::time::{Duration, Instant};
use tiberius::Config;

pub const DEFAULT_PROBE_TIMEOUT: Duration = Duration::from_secs(2);

#[async_trait]
pub trait MssqlHealthcheckOps: Send + Sync {
    async fn ping(&self, conn_str: &str) -> Result<(), String>;
}

pub(super) struct MssqlClientHealthcheckOps;

impl MssqlClientHealthcheckOps {
    pub(super) fn new() -> Self {
        Self
    }
}

#[async_trait]
impl MssqlHealthcheckOps for MssqlClientHealthcheckOps {
    async fn ping(&self, conn_str: &str) -> Result<(), String> {
        let config = Config::from_ado_string(conn_str)
            .map_err(|e| format!("parse ADO connection string: {e}"))?;

        let mut client = super::mssql_container_impl::connect_with_config(config)
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
    overall_timeout_ms: u64,
    attempt_budget: Option<Duration>,
) -> Result<(), String> {
    let overall = Duration::from_millis(overall_timeout_ms);
    let poll_every = Duration::from_millis(250);

    tracing::info!(
        subsystem = "mssql",
        dependency = identifier,
        overall = ?overall,
        attempt_budget = ?attempt_budget,
        "readiness probe loop starting"
    );

    let start = Instant::now();
    let mut attempt: u64 = 0;

    loop {
        if start.elapsed() >= overall {
            return Err(format!(
                "[MssqlDependency-{}] mssql did not become ready within {:?}. connection_string={:?}, attempts={}",
                identifier, overall, connection_string, attempt
            ));
        }

        attempt = attempt.saturating_add(1);
        let attempt_started = Instant::now();

        let attempt_result = match attempt_budget {
            Some(budget) => match tokio::time::timeout(budget, ops.ping(connection_string)).await {
                Ok(inner) => inner,
                Err(_) => Err(format!("attempt exceeded {budget:?}")),
            },
            None => ops.ping(connection_string).await,
        };

        match attempt_result {
            Ok(()) => {
                tracing::info!(
                    subsystem = "mssql",
                    dependency = identifier,
                    attempts = attempt,
                    elapsed_total = ?start.elapsed(),
                    "readiness probe succeeded"
                );
                return Ok(());
            }
            Err(err) => tracing::info!(
                subsystem = "mssql",
                dependency = identifier,
                attempt = attempt,
                elapsed = ?attempt_started.elapsed(),
                error = %err,
                "readiness probe failed (will retry)"
            ),
        }

        tokio::time::sleep(poll_every).await;
    }
}

pub struct DefaultMssqlReadinessCheck {
    probe_timeout: Option<Duration>,
}

impl DefaultMssqlReadinessCheck {
    pub fn new() -> Self {
        Self {
            probe_timeout: Some(DEFAULT_PROBE_TIMEOUT),
        }
    }

    pub fn with_probe_timeout(mut self, probe_timeout: Option<Duration>) -> Self {
        self.probe_timeout = probe_timeout;
        self
    }

    pub fn probe_timeout(&self) -> Option<Duration> {
        self.probe_timeout
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
        let ops = MssqlClientHealthcheckOps::new();
        run_with_retry(
            &ops,
            identifier,
            connection_string,
            timeout_ms,
            self.probe_timeout,
        )
        .await
    }
}
