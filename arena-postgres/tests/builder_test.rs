use arena::dependency::RunnableDependency;
use arena::healthcheck::ReadinessCheck;
use arena_postgres::{PostgresDependency, PostgresImpl};
use async_trait::async_trait;

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
    ) -> Result<(), String> {
        self.conn_str = Some("postgres://127.0.0.1:5432/fake".to_string());
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), String> {
        self.conn_str = None;
        Ok(())
    }
    async fn force_stop(&mut self) -> bool {
        true
    }
    fn release(&mut self) {}


    fn connection_string(&self) -> Option<&str> {
        self.conn_str.as_deref()
    }
}

struct FakeReadinessCheck;

#[async_trait]
impl ReadinessCheck for FakeReadinessCheck {
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
fn builder_defaults_produce_expected_identifier() {
    let dep = PostgresDependency::builder("defaults").build();
    assert!(dep.identifier().contains("defaults"));
    assert!(dep.identifier().starts_with("arena-postgres-"));
}

#[test]
fn builder_with_container_name_and_network_builds_ok() {
    let dep = PostgresDependency::builder("named")
        .with_impl(FakePostgresImpl { conn_str: None })
        .with_port(5555)
        .with_database_name("custom_db")
        .with_database_username("custom_user")
        .with_database_password("custom_pass")
        .with_container_name("custom-container")
        .with_network("custom-network")
        .with_image_name("custom/image")
        .with_image_tag("1.2.3")
        .build();

    assert!(dep.identifier().contains("named"));
    assert_eq!(dep.connection_string(), None);
}

#[test]
fn builder_with_image_sets_image_tag() {
    let dep = PostgresDependency::builder("image-alias")
        .with_impl(FakePostgresImpl { conn_str: None })
        .with_image("15-alpine")
        .build();

    assert!(dep.identifier().contains("image-alias"));
}

#[test]
fn builder_with_container_tag_sets_image_tag() {
    let dep = PostgresDependency::builder("container-tag-alias")
        .with_impl(FakePostgresImpl { conn_str: None })
        .with_container_tag("16-alpine")
        .build();

    assert!(dep.identifier().contains("container-tag-alias"));
}

#[test]
fn builder_with_child_dependencies_populates_children() {
    let dep = PostgresDependency::builder("with-children")
        .with_impl(FakePostgresImpl { conn_str: None })
        .with_child_dependencies(Vec::new())
        .build();

    assert!(dep.children().is_empty());
}

#[tokio::test]
async fn builder_with_readiness_check_used_on_start() {
    let mut dep = PostgresDependency::builder("readiness-override")
        .with_impl(FakePostgresImpl { conn_str: None })
        .with_readiness_check(FakeReadinessCheck)
        .build();

    dep.start().await.expect("start should succeed");
    assert_eq!(dep.connection_string(), Some("postgres://127.0.0.1:5432/fake"));
    dep.stop().await.expect("stop should succeed");
}

#[derive(Clone, Default)]
struct ExpiryRecordingImpl {
    expiry: std::sync::Arc<std::sync::Mutex<Option<Option<std::time::Duration>>>>,
}

#[async_trait]
impl PostgresImpl for ExpiryRecordingImpl {
    fn set_expiry(&mut self, expiry: Option<std::time::Duration>) {
        *self.expiry.lock().unwrap() = Some(expiry);
    }

    async fn start(
        &mut self,
        _port: u16,
        _database_name: &str,
        _database_username: &str,
        _database_password: &str,
        _image_name: &str,
        _image_tag: &str,
        _container_name: &str,
    ) -> Result<(), String> {
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), String> {
        Ok(())
    }
    async fn force_stop(&mut self) -> bool {
        true
    }
    fn release(&mut self) {}

    fn connection_string(&self) -> Option<&str> {
        None
    }
}

#[test]
fn build_without_expiry_override_uses_the_default_expiry() {
    let recorder = ExpiryRecordingImpl::default();
    let _dep = PostgresDependency::builder("orders")
        .with_impl(recorder.clone())
        .build();

    assert_eq!(
        *recorder.expiry.lock().unwrap(),
        Some(Some(arena_container::expiry::DEFAULT_EXPIRY))
    );
}

#[test]
fn build_with_expiry_uses_the_given_expiry() {
    let recorder = ExpiryRecordingImpl::default();
    let _dep = PostgresDependency::builder("orders")
        .with_impl(recorder.clone())
        .with_expiry(std::time::Duration::from_secs(30))
        .build();

    assert_eq!(
        *recorder.expiry.lock().unwrap(),
        Some(Some(std::time::Duration::from_secs(30)))
    );
}

#[test]
fn build_without_expiry_disables_expiry() {
    let recorder = ExpiryRecordingImpl::default();
    let _dep = PostgresDependency::builder("orders")
        .with_impl(recorder.clone())
        .without_expiry()
        .build();

    assert_eq!(*recorder.expiry.lock().unwrap(), Some(None));
}
