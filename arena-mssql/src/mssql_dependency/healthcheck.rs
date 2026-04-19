use arena::healthcheck::ReadinessCheck;
use async_trait::async_trait;
use std::time::{Duration, Instant};

#[async_trait]
pub trait MssqlHealthcheckOps: Send + Sync {
    async fn ping(&self, conn_str: &str) -> Result<(), String>;
}

pub(super) struct MssqlClientHealthcheckOps;

#[async_trait]
impl MssqlHealthcheckOps for MssqlClientHealthcheckOps {
    async fn ping(&self, conn_str: &str) -> Result<(), String> {
        let mut client = super::mssql_container_impl::connect(conn_str)
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
    let poll_every = Duration::from_millis(500);

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
            Err(err) => log::debug!("[Mssql] healthcheck failed (will retry): {err}"),
        };

        tokio::time::sleep(poll_every).await;
    }
}

pub(super) struct DefaultMssqlReadinessCheck;

#[async_trait]
impl ReadinessCheck for DefaultMssqlReadinessCheck {
    async fn is_ready(
        &self,
        identifier: &str,
        connection_string: &str,
        timeout_ms: u64,
    ) -> Result<(), String> {
        let ops = MssqlClientHealthcheckOps;
        run_with_retry(&ops, identifier, connection_string, timeout_ms).await
    }
}
