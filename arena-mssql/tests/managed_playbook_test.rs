use arena::dependency::{Dependency, RunnableDependency};
use arena::Playbook;
use arena_mssql::{ManagedMssqlPlaybook, MssqlDependency};
use futures::FutureExt;

#[test]
fn identifier_configured_value_returns_it() {
    let playbook = ManagedMssqlPlaybook::new("my-playbook", "mssql-dep");
    assert_eq!(playbook.identifier(), "my-playbook");
}

#[test]
fn into_box_configured_value_preserves_identifier() {
    let boxed = ManagedMssqlPlaybook::new("boxed-playbook", "mssql-dep").into_box();
    assert_eq!(boxed.identifier(), "boxed-playbook");
}

#[tokio::test]
async fn run_missing_dependency_panics() {
    let playbook = ManagedMssqlPlaybook::new("missing", "does-not-exist");
    let deps: Vec<Dependency> = Vec::new();

    let outcome = std::panic::AssertUnwindSafe(playbook.run(&deps))
        .catch_unwind()
        .await;

    assert!(outcome.is_err());
}

struct OtherDependency;

#[async_trait::async_trait]
impl RunnableDependency for OtherDependency {
    fn identifier(&self) -> &str {
        "other-dep"
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
async fn run_dependency_wrong_type_panics() {
    let playbook = ManagedMssqlPlaybook::new("wrong-type", "other-dep");
    let deps: Vec<Dependency> = vec![Box::new(OtherDependency)];

    let outcome = std::panic::AssertUnwindSafe(playbook.run(&deps))
        .catch_unwind()
        .await;

    assert!(outcome.is_err());
}

#[tokio::test]
async fn run_mssql_not_started_panics() {
    let mssql = MssqlDependency::builder("mssql-for-playbook").build();
    let dependency_identifier = mssql.identifier().to_string();
    let dep: Box<dyn RunnableDependency> = Box::new(mssql);
    let deps: Vec<Dependency> = vec![dep];

    let playbook = ManagedMssqlPlaybook::new("unstarted", dependency_identifier);

    let outcome = std::panic::AssertUnwindSafe(playbook.run(&deps))
        .catch_unwind()
        .await;

    assert!(outcome.is_err());
}
