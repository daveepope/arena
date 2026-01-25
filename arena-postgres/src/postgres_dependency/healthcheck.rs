use arena::healthcheck::ReadinessCheck;
use async_trait::async_trait;
use backon::{BlockingRetryable, ConstantBuilder};
use futures::channel::oneshot;
use std::time::{Duration, Instant};

pub(super) struct PostgresDefaultReadinessCheck;

impl PostgresDefaultReadinessCheck {
    fn is_ready_once(conn_str: &str) -> bool {
        let mut client = match postgres::Client::connect(conn_str, postgres::NoTls) {
            Ok(v) => v,
            Err(_) => return false,
        };

        client.simple_query("SELECT 1").is_ok()
    }
}

#[async_trait]
impl ReadinessCheck for PostgresDefaultReadinessCheck {
    async fn is_ready(
        &self,
        identifier: &str,
        target: &str,
        timeout: Duration,
    ) -> Result<(), String> {
        let identifier = identifier.to_string();
        let identifier_for_thread = identifier.clone();
        let conn_str_for_thread = target.to_string();
        let (tx, rx) = oneshot::channel::<Result<(), String>>();

        std::thread::spawn(move || {
            let poll_every = Duration::from_millis(250);
            let start = Instant::now();

            let policy = ConstantBuilder::default()
                .with_delay(poll_every)
                .without_max_times();

            let is_ready_once = || {
                if PostgresDefaultReadinessCheck::is_ready_once(&conn_str_for_thread) {
                    Ok(())
                } else {
                    Err(())
                }
            };

            let result = is_ready_once
                .retry(policy)
                .sleep(std::thread::sleep)
                .when(|_| start.elapsed() < timeout)
                .call();

            match result {
                Ok(()) => {
                    let _ = tx.send(Ok(()));
                }
                Err(()) => {
                    let _ = tx.send(Err(format!(
                        "[PostgresDependency-{}] postgres did not become ready within {:?}. target={:?}",
                        identifier_for_thread, timeout, conn_str_for_thread
                    )));
                }
            }
        });

        match rx.await {
            Ok(v) => v,
            Err(_canceled) => Err(format!(
                "[PostgresDependency-{}] readiness/health-check worker thread unexpectedly stopped.",
                identifier
            )),
        }
    }
}

