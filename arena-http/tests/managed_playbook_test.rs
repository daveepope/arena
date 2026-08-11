use arena::dependency::{Dependency, RunnableDependency};
use arena::playbook::Playbook as PlaybookTrait;
use arena_http::{HttpDependency, HttpImpl, ManagedHttpPlaybook};
use async_trait::async_trait;

struct FakeHttpImpl {
    base_url: Option<String>,
}

#[async_trait]
impl HttpImpl for FakeHttpImpl {
    async fn start(&mut self, _port: u16, _image_name: &str, _image_tag: &str, _container_name: &str) {
        self.base_url = Some("http://127.0.0.1:8080".to_string());
    }

    async fn stop(&mut self) {
        self.base_url = None;
    }

    fn base_url(&self) -> Option<&str> {
        self.base_url.as_deref()
    }

    fn admin_url(&self) -> Option<String> {
        self.base_url.as_deref().map(|url| format!("{url}/__admin"))
    }
}

struct OkReadinessCheck;

#[async_trait]
impl arena::healthcheck::ReadinessCheck for OkReadinessCheck {
    async fn is_ready(&self, _identifier: &str, _admin_url: &str, _timeout_ms: u64) -> Result<(), String> {
        Ok(())
    }
}

async fn started_http(identifier: &str) -> HttpDependency {
    let mut dep = HttpDependency::builder(identifier)
        .with_impl(FakeHttpImpl { base_url: None })
        .with_port(0)
        .with_readiness_check(OkReadinessCheck)
        .build();
    dep.start().await;
    dep
}

#[test]
fn identifier_returns_configured_value() {
    let playbook = ManagedHttpPlaybook::new("managed-id", "http-dep", |p| p);
    assert_eq!(
        arena::playbook::Playbook::identifier(&playbook),
        "managed-id"
    );
}

#[tokio::test]
async fn run_dependency_present_applies_build_fn() {
    let dep = started_http("http-managed").await;
    let dependency_identifier = dep.identifier().to_string();
    let deps: Vec<Dependency> = vec![Box::new(dep)];

    let playbook = ManagedHttpPlaybook::new("managed-run", dependency_identifier, |p| p);

    let active = playbook.run(&deps).await;
    assert_eq!(active.identifier(), "managed-run");
}

#[tokio::test]
#[should_panic(expected = "not found or is not an HttpDependency")]
async fn run_dependency_missing_panics() {
    let deps: Vec<Dependency> = Vec::new();
    let playbook = ManagedHttpPlaybook::new("managed-missing", "no-such-dep", |p| p);
    let _ = playbook.run(&deps).await;
}

#[test]
fn into_box_produces_trait_object() {
    let playbook = ManagedHttpPlaybook::new("boxed", "http-dep", |p| p);
    let boxed: Box<dyn PlaybookTrait> = playbook.into_box();
    assert_eq!(boxed.identifier(), "boxed");
}
