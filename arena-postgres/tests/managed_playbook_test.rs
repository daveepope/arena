use arena::lifecycle::{Fault, RunnableState, Subject};
use arena::dependency::{Dependency, RunnableDependency};
use arena::Playbook;
use arena_postgres::{ManagedPostgresPlaybook, PostgresDependency};

#[test]
fn identifier_returns_configured_value() {
    let playbook = ManagedPostgresPlaybook::new("my-playbook", "postgres-dep");
    assert_eq!(playbook.identifier(), "my-playbook");
}

#[test]
fn into_box_preserves_identifier() {
    let boxed = ManagedPostgresPlaybook::new("boxed-playbook", "postgres-dep").into_box();
    assert_eq!(boxed.identifier(), "boxed-playbook");
}

#[tokio::test]
async fn run_missing_dependency_returns_fault() {
    let playbook = ManagedPostgresPlaybook::new("missing", "does-not-exist");
    let deps: Vec<Dependency> = Vec::new();

    let Err(fault) = playbook.run(&deps).await else {
        panic!("playbook should fault");
    };

    assert_eq!(fault.subject, Subject::Playbook);
    assert_eq!(fault.id, playbook.identifier());
}

#[tokio::test]
async fn run_dependency_present_but_not_postgres_returns_fault() {
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

    let playbook = ManagedPostgresPlaybook::new("wrong-type", "other-dep");
    let deps: Vec<Dependency> = vec![Box::new(OtherDependency)];

    let Err(fault) = playbook.run(&deps).await else {
        panic!("playbook should fault");
    };

    assert_eq!(fault.subject, Subject::Playbook);
    assert_eq!(fault.id, playbook.identifier());
}

#[tokio::test]
async fn run_postgres_not_started_returns_fault() {
    let pg = PostgresDependency::builder("postgres-for-playbook").build();
    let dependency_identifier = pg.identifier().to_string();
    let dep: Box<dyn RunnableDependency> = Box::new(pg);
    let deps: Vec<Dependency> = vec![dep];

    let playbook = ManagedPostgresPlaybook::new("unstarted", dependency_identifier);

    let Err(fault) = playbook.run(&deps).await else {
        panic!("playbook should fault");
    };

    assert_eq!(fault.subject, Subject::Playbook);
    assert_eq!(fault.id, playbook.identifier());
}
