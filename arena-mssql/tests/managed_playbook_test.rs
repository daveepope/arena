use arena::lifecycle::{Fault, RunnableState, Subject};
use arena::dependency::{Dependency, RunnableDependency};
use arena::Playbook;
use arena_mssql::{ManagedMssqlPlaybook, MssqlDependency};

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
async fn run_missing_dependency_returns_fault() {
    let playbook = ManagedMssqlPlaybook::new("missing", "does-not-exist");
    let deps: Vec<Dependency> = Vec::new();

    let Err(fault) = playbook.run(&deps).await else {
        panic!("playbook should fault");
    };

    assert_eq!(fault.subject, Subject::Playbook);
    assert_eq!(fault.id, playbook.identifier());
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
    fn state(&self) -> RunnableState {
        RunnableState::NotStarted
    }

    fn faults(&self) -> &[Fault] {
        &[]
    }

    async fn force_stop(&mut self) {}
    fn release(&mut self) {}


    async fn start(&mut self) -> Result<(), Fault> {
        Ok(())
    }
    async fn stop(&mut self) -> Result<(), Fault> {
        Ok(())
    }
    fn add_child(&mut self, _dep: Box<dyn RunnableDependency>) {}
    fn children(&self) -> &[Dependency] {
        &[]
    }
    fn children_mut(&mut self) -> &mut [Dependency] {
        &mut []
    }
    async fn soft_reset(&self) -> Result<(), Fault> {
        Ok(())
    }
    async fn hard_reset(&mut self) -> Result<(), Fault> {
        Ok(())
    }
}

#[tokio::test]
async fn run_dependency_wrong_type_returns_fault() {
    let playbook = ManagedMssqlPlaybook::new("wrong-type", "other-dep");
    let deps: Vec<Dependency> = vec![Box::new(OtherDependency)];

    let Err(fault) = playbook.run(&deps).await else {
        panic!("playbook should fault");
    };

    assert_eq!(fault.subject, Subject::Playbook);
    assert_eq!(fault.id, playbook.identifier());
}

#[tokio::test]
async fn run_mssql_not_started_returns_fault() {
    let mssql = MssqlDependency::builder("mssql-for-playbook").build();
    let dependency_identifier = mssql.identifier().to_string();
    let dep: Box<dyn RunnableDependency> = Box::new(mssql);
    let deps: Vec<Dependency> = vec![dep];

    let playbook = ManagedMssqlPlaybook::new("unstarted", dependency_identifier);

    let Err(fault) = playbook.run(&deps).await else {
        panic!("playbook should fault");
    };

    assert_eq!(fault.subject, Subject::Playbook);
    assert_eq!(fault.id, playbook.identifier());
}
