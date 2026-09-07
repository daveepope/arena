use arena::lifecycle::{Fault, RunnableState, Subject};
use arena::dependency::{Dependency, RunnableDependency};
use arena::healthcheck::ReadinessCheck;
use arena::Playbook as _;
use arena_localstack::{LocalstackDependency, LocalstackImpl, ManagedLocalstackPlaybook};
use async_trait::async_trait;

struct FakeLocalstackImpl {
    endpoint: Option<String>,
}

#[async_trait]
impl LocalstackImpl for FakeLocalstackImpl {
    async fn start(
        &mut self,
        _port: u16,
        _image_name: &str,
        _image_tag: &str,
        _container_name: &str,
        _services: &[String],
    ) -> Result<(), String> {
        self.endpoint = Some("http://127.0.0.1:4566".to_string());
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), String> {
        Ok(())
    }
    async fn force_stop(&mut self) -> bool {
        true
    }
    fn release(&mut self) {}


    fn endpoint_url(&self) -> Option<&str> {
        self.endpoint.as_deref()
    }
}

struct OkReadinessCheck;

#[async_trait]
impl ReadinessCheck for OkReadinessCheck {
    async fn is_ready(
        &self,
        _identifier: &str,
        _endpoint: &str,
        _timeout_ms: u64,
    ) -> Result<(), String> {
        Ok(())
    }
}

struct OtherDependency;

#[async_trait]
impl RunnableDependency for OtherDependency {
    fn identifier(&self) -> &str {
        "not-a-localstack"
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

async fn started_localstack(identifier: &str) -> LocalstackDependency {
    let mut dep = LocalstackDependency::builder(identifier)
        .with_impl(FakeLocalstackImpl { endpoint: None })
        .with_port(0)
        .with_image_tag("x")
        .with_readiness_check(OkReadinessCheck)
        .build();
    dep.start().await.expect("start should succeed");
    dep
}

#[tokio::test]
async fn run_dependency_found_returns_active_playbook_with_identifier() {
    let dep = started_localstack("localstack-managed").await;
    let dep_identifier = dep.identifier().to_string();
    let deps: Vec<Dependency> = vec![Box::new(dep)];

    let managed = ManagedLocalstackPlaybook::new("managed-playbook-id", dep_identifier);

    let active = managed.run(&deps).await.expect("playbook should run");

    assert_eq!(active.identifier(), "managed-playbook-id");
    assert!(active.as_any().is::<arena_localstack::ActivePlaybook>());
}

#[tokio::test]
async fn run_dependency_missing_returns_fault() {
    let deps: Vec<Dependency> = vec![Box::new(OtherDependency)];
    let managed = ManagedLocalstackPlaybook::new("managed-playbook-id", "does-not-exist");

    let Err(fault) = managed.run(&deps).await else {
        panic!("playbook should fault");
    };

    assert_eq!(fault.subject, Subject::Playbook);
    assert_eq!(fault.id, managed.identifier());
}

#[tokio::test]
async fn run_dependency_wrong_type_returns_fault() {
    let deps: Vec<Dependency> = vec![Box::new(OtherDependency)];
    let managed = ManagedLocalstackPlaybook::new("managed-playbook-id", "not-a-localstack");

    let Err(fault) = managed.run(&deps).await else {
        panic!("playbook should fault");
    };

    assert_eq!(fault.subject, Subject::Playbook);
    assert_eq!(fault.id, managed.identifier());
}

#[test]
fn into_box_wraps_playbook_trait_object() {
    let managed = ManagedLocalstackPlaybook::new("boxed-id", "target-dep");

    let boxed = managed.into_box();

    assert_eq!(boxed.identifier(), "boxed-id");
}
