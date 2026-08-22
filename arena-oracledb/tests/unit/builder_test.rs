use arena::dependency::{Dependency, RunnableDependency};
use arena::healthcheck::ReadinessCheck;
use arena_oracledb::{OracleDependency, OracleImpl};
use async_trait::async_trait;

struct FakeOracleImpl;

#[async_trait]
impl OracleImpl for FakeOracleImpl {
    #[allow(clippy::too_many_arguments)]
    async fn start(
        &self,
        _port: u16,
        _database_name: &str,
        _database_username: &str,
        _database_password: &str,
        _admin_password: &str,
        _image_name: &str,
        _image_tag: &str,
        _container_name: &str,
    ) {
    }

    async fn stop(&self) {}

    fn connection_string(&self) -> Option<String> {
        None
    }

    fn host_address(&self) -> Option<String> {
        None
    }

    async fn run_sqlplus(&self, _username: &str, _password: &str, _script: &str) -> Result<String, String> {
        Ok(String::new())
    }
}

struct FakeReadinessCheck;

#[async_trait]
impl ReadinessCheck for FakeReadinessCheck {
    async fn is_ready(&self, _identifier: &str, _target: &str, _timeout_ms: u64) -> Result<(), String> {
        Ok(())
    }
}

struct NoopChildDependency {
    identifier: String,
}

#[async_trait]
impl RunnableDependency for NoopChildDependency {
    fn identifier(&self) -> &str {
        &self.identifier
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    async fn start(&mut self) {}
    async fn stop(&mut self) {}

    fn add_child(&mut self, _dep: Box<dyn RunnableDependency>) {}

    fn children(&self) -> &[Dependency] {
        &[]
    }

    fn children_mut(&mut self) -> &mut [Dependency] {
        &mut []
    }

    async fn soft_reset(&self) {}
    async fn hard_reset(&mut self) {}
}

#[test]
fn build_defaults_sets_expected_database_identity() {
    let dep = OracleDependency::builder("defaults-test").build();

    assert_eq!(dep.database_name(), "FREEPDB1");
    assert_eq!(dep.database_username(), "arena_user");
    assert!(dep.identifier.starts_with("arena-oracledb-defaults-test-"));
}

#[test]
fn with_database_name_overrides_default() {
    let dep = OracleDependency::builder("custom-db")
        .with_database_name("CUSTOMPDB")
        .build();

    assert_eq!(dep.database_name(), "CUSTOMPDB");
}

#[test]
fn with_database_username_overrides_default() {
    let dep = OracleDependency::builder("custom-user")
        .with_database_username("custom_user")
        .build();

    assert_eq!(dep.database_username(), "custom_user");
}

#[test]
fn with_impl_injects_custom_implementation() {
    let dep = OracleDependency::builder("custom-impl")
        .with_impl(FakeOracleImpl)
        .build();

    assert_eq!(dep.connection_string(), None);
}

#[test]
fn with_readiness_check_accepts_custom_check() {
    let _dep = OracleDependency::builder("custom-readiness")
        .with_impl(FakeOracleImpl)
        .with_readiness_check(FakeReadinessCheck)
        .build();
}

#[test]
fn with_image_name_and_tag_are_accepted() {
    let _dep = OracleDependency::builder("custom-image")
        .with_image_name("custom/oracle-free")
        .with_image_tag("23-slim")
        .build();
}

#[test]
fn with_image_sets_tag_only() {
    let _dep = OracleDependency::builder("custom-image-tag")
        .with_image("23-slim")
        .build();
}

#[test]
fn with_container_tag_sets_tag_only() {
    let _dep = OracleDependency::builder("custom-container-tag")
        .with_container_tag("23-slim")
        .build();
}

#[test]
fn with_container_name_and_network_are_accepted() {
    let _dep = OracleDependency::builder("custom-container")
        .with_container_name("my-oracle-container")
        .with_network("my-network")
        .build();
}

#[test]
fn with_admin_password_overrides_default() {
    let _dep = OracleDependency::builder("custom-admin-password")
        .with_admin_password("SuperSecret1!")
        .build();
}

#[test]
fn with_startup_sql_scripts_are_accepted() {
    let _dep = OracleDependency::builder("custom-scripts")
        .with_startup_sql_scripts(vec!["CREATE TABLE widgets (id NUMBER);".to_string()])
        .build();
}

#[test]
fn with_child_dependencies_are_accepted() {
    let child: Box<dyn RunnableDependency> = Box::new(NoopChildDependency {
        identifier: "child".to_string(),
    });

    let dep = OracleDependency::builder("custom-children")
        .with_child_dependencies(vec![child])
        .build();

    assert_eq!(dep.children().len(), 1);
}

#[test]
fn with_port_is_accepted() {
    let _dep = OracleDependency::builder("custom-port").with_port(15210).build();
}
