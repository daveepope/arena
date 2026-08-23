use arena::dependency::{Dependency, RunnableDependency};
use arena::playbook::Playbook as PlaybookTrait;
use arena_oracledb::{ManagedOraclePlaybook, OracleDependency, OracleImpl};
use async_trait::async_trait;

struct FakeStartedOracleImpl;

#[async_trait]
impl OracleImpl for FakeStartedOracleImpl {
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
        Some("//localhost:1521/FREEPDB1".to_string())
    }

    fn host_address(&self) -> Option<String> {
        Some("127.0.0.1:1521".to_string())
    }

    async fn run_sqlplus(&self, _username: &str, _password: &str, _script: &str) -> Result<String, String> {
        Ok(String::new())
    }
}

struct OtherDependency {
    identifier: String,
}

#[async_trait]
impl RunnableDependency for OtherDependency {
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

#[tokio::test]
#[should_panic(expected = "not found or is not an OracleDependency")]
async fn run_missing_dependency_panics() {
    let managed = ManagedOraclePlaybook::new("managed-1", "does-not-exist");
    let deps: Vec<Dependency> = vec![];
    let _ = managed.run(&deps).await;
}

#[tokio::test]
#[should_panic(expected = "not found or is not an OracleDependency")]
async fn run_wrong_type_dependency_panics() {
    let managed = ManagedOraclePlaybook::new("managed-2", "other-dep");
    let deps: Vec<Dependency> = vec![Box::new(OtherDependency {
        identifier: "other-dep".to_string(),
    })];
    let _ = managed.run(&deps).await;
}

#[tokio::test]
async fn run_success_delegates_to_oracle_playbook() {
    let dependency = OracleDependency::builder("managed-success").with_impl(FakeStartedOracleImpl).build();
    let dependency_identifier = dependency.identifier.clone();
    let dep: Box<dyn RunnableDependency> = Box::new(dependency);
    let deps: Vec<Dependency> = vec![dep];

    let managed = ManagedOraclePlaybook::new("managed-playbook-id", dependency_identifier);
    let active = managed.run(&deps).await;

    assert_eq!(active.identifier(), "managed-playbook-id");
}

#[test]
fn into_box_returns_boxed_trait_object_with_identifier() {
    let managed = ManagedOraclePlaybook::new("managed-3", "some-dep");

    let boxed: Box<dyn PlaybookTrait> = managed.into_box();

    assert_eq!(boxed.identifier(), "managed-3");
}
