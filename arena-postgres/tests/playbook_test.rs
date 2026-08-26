use arena::dependency::RunnableDependency;
use arena::healthcheck::ReadinessCheck;
use arena_postgres::{PostgresDependency, PostgresImpl};
use async_trait::async_trait;
use futures::FutureExt;

struct FakePostgresImpl {
    conn_str: Option<String>,
}

#[async_trait]
impl PostgresImpl for FakePostgresImpl {
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
        self.conn_str = Some("postgres://u:p@127.0.0.1:1/fake".to_string());
    }

    async fn stop(&mut self) {
        self.conn_str = None;
    }

    fn connection_string(&self) -> Option<&str> {
        self.conn_str.as_deref()
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

#[test]
fn with_before_start_panics() {
    let dep = PostgresDependency::builder("playbook-before-start").build();

    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| dep.playbook()));

    assert!(outcome.is_err());
}

#[tokio::test]
async fn with_identifier_overrides_default_identifier() {
    let mut dep = PostgresDependency::builder("playbook-with-identifier")
        .with_impl(FakePostgresImpl { conn_str: None })
        .with_readiness_check(AlwaysOkReadinessCheck)
        .build();
    dep.start().await;

    let playbook = dep.playbook().with_identifier("custom-playbook-id");

    let outcome = std::panic::AssertUnwindSafe(playbook.run()).catch_unwind().await;

    assert!(outcome.is_err());

    dep.stop().await;
}
