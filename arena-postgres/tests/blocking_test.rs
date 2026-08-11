use arena::dependency::RunnableDependency;
use arena::healthcheck::ReadinessCheck;
use arena_postgres::{PostgresDependency, PostgresImpl};
use async_trait::async_trait;
use futures::FutureExt;

struct RefusedPortPostgresImpl;

#[async_trait]
impl PostgresImpl for RefusedPortPostgresImpl {
    async fn start(
        &mut self,
        _port: u16,
        _database_name: &str,
        _database_username: &str,
        _database_password: &str,
        _image_name: &str,
        _image_tag: &str,
        _container_name: &str,
    ) {
    }

    async fn stop(&mut self) {}

    fn connection_string(&self) -> Option<&str> {
        Some("postgres://u:p@127.0.0.1:1/db")
    }
}

struct AlwaysOkReadinessCheck;

#[async_trait]
impl ReadinessCheck for AlwaysOkReadinessCheck {
    async fn is_ready(
        &self,
        _identifier: &str,
        _connection_string: &str,
        _timeout_ms: u64,
    ) -> Result<(), String> {
        Ok(())
    }
}

#[tokio::test]
async fn start_startup_script_connect_failure_panics_on_blocking_thread() {
    let mut dep = PostgresDependency::builder("postgres-refused")
        .with_impl(RefusedPortPostgresImpl)
        .with_readiness_check(AlwaysOkReadinessCheck)
        .with_startup_sql_scripts(vec!["select 1;".to_string()])
        .build();

    let outcome = std::panic::AssertUnwindSafe(async {
        dep.start().await;
    })
    .catch_unwind()
    .await;

    assert!(outcome.is_err());
}

#[tokio::test]
async fn playbook_run_connect_failure_panics_on_blocking_thread() {
    let mut dep = PostgresDependency::builder("postgres-refused-playbook")
        .with_impl(RefusedPortPostgresImpl)
        .with_readiness_check(AlwaysOkReadinessCheck)
        .build();

    dep.start().await;

    let outcome = std::panic::AssertUnwindSafe(async { dep.playbook().run().await })
        .catch_unwind()
        .await;

    assert!(outcome.is_err());
}
