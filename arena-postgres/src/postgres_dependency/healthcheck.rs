use arena::healthcheck::ReadinessCheck;
use async_trait::async_trait;
use futures::channel::oneshot;
use std::time::{Duration, Instant};

pub trait PostgresHealthcheckOps: Send + Sync {
    fn ping(&self, conn_str: &str) -> Result<(), String>;
}

pub(super) struct PostgresClientHealthcheckOps;

impl PostgresHealthcheckOps for PostgresClientHealthcheckOps {
    fn ping(&self, conn_str: &str) -> Result<(), String> {
        let mut client = postgres::Client::connect(conn_str, postgres::NoTls)
            .map_err(|err| format!("postgres connect failed: {err}"))?;

        client
            .simple_query("SELECT 1")
            .map(|_res| ())
            .map_err(|err| format!("postgres ping query failed: {err}"))
    }
}

fn run_with_retry_blocking(
    ops: &impl PostgresHealthcheckOps,
    identifier: &str,
    connection_string: &str,
    timeout_ms: u64,
) -> Result<(), String> {
    let timeout_duration = Duration::from_millis(timeout_ms);

    #[cfg(test)]
    let poll_every = Duration::from_millis(1);
    #[cfg(not(test))]
    let poll_every = Duration::from_millis(100);

    let start = Instant::now();
    loop {
        if start.elapsed() >= timeout_duration {
            return Err(format!(
                "[PostgresDependency-{}] postgres did not become ready within {:?}. connection_string={:?}",
                identifier, timeout_duration, connection_string
            ));
        }

        match ops.ping(connection_string) {
            Ok(()) => return Ok(()),
            Err(err) => log::debug!("[Postgres] healthcheck failed (will retry): {err}"),
        };

        std::thread::sleep(poll_every);
    }
}

pub(super) struct DefaultPostgresReadinessCheck;

#[async_trait]
impl ReadinessCheck for DefaultPostgresReadinessCheck {
    async fn is_ready(
        &self,
        identifier: &str,
        connection_string: &str,
        timeout_ms: u64,
    ) -> Result<(), String> {
        let identifier = identifier.to_string();
        let connection_string = connection_string.to_string();
        let (tx, rx) = oneshot::channel::<Result<(), String>>();

        std::thread::spawn(move || {
            let ops = PostgresClientHealthcheckOps;
            let res = run_with_retry_blocking(&ops, &identifier, &connection_string, timeout_ms);
            let _ = tx.send(res);
        });

        rx.await.map_err(|_canceled| {
            "[PostgresDependency] readiness/health-check worker thread unexpectedly stopped."
                .to_string()
        })?
    }
}
