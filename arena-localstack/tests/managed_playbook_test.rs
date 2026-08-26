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
    ) {
        self.endpoint = Some("http://127.0.0.1:4566".to_string());
    }

    async fn stop(&mut self) {}

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

async fn started_localstack(identifier: &str) -> LocalstackDependency {
    let mut dep = LocalstackDependency::builder(identifier)
        .with_impl(FakeLocalstackImpl { endpoint: None })
        .with_port(0)
        .with_image_tag("x")
        .with_readiness_check(OkReadinessCheck)
        .build();
    dep.start().await;
    dep
}

#[tokio::test]
async fn run_dependency_found_returns_active_playbook_with_identifier() {
    let dep = started_localstack("localstack-managed").await;
    let dep_identifier = dep.identifier().to_string();
    let deps: Vec<Dependency> = vec![Box::new(dep)];

    let managed = ManagedLocalstackPlaybook::new("managed-playbook-id", dep_identifier);

    let active = managed.run(&deps).await;

    assert_eq!(active.identifier(), "managed-playbook-id");
    assert!(active.as_any().is::<arena_localstack::ActivePlaybook>());
}

#[tokio::test]
async fn run_dependency_missing_panics() {
    let deps: Vec<Dependency> = vec![Box::new(OtherDependency)];
    let managed = ManagedLocalstackPlaybook::new("managed-playbook-id", "does-not-exist");

    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        futures::executor::block_on(managed.run(&deps));
    }));

    assert!(outcome.is_err());
}

#[tokio::test]
async fn run_dependency_wrong_type_panics() {
    let deps: Vec<Dependency> = vec![Box::new(OtherDependency)];
    let managed = ManagedLocalstackPlaybook::new("managed-playbook-id", "not-a-localstack");

    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        futures::executor::block_on(managed.run(&deps));
    }));

    assert!(outcome.is_err());
}

#[test]
fn into_box_wraps_playbook_trait_object() {
    let managed = ManagedLocalstackPlaybook::new("boxed-id", "target-dep");

    let boxed = managed.into_box();

    assert_eq!(boxed.identifier(), "boxed-id");
}
