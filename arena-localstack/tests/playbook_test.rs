use arena::dependency::RunnableDependency;
use arena_localstack::{LocalstackDependency, LocalstackImpl};
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
impl arena::healthcheck::ReadinessCheck for OkReadinessCheck {
    async fn is_ready(
        &self,
        _identifier: &str,
        _endpoint: &str,
        _timeout_ms: u64,
    ) -> Result<(), String> {
        Ok(())
    }
}

#[test]
fn with_unstarted_dep_panics() {
    let dep = LocalstackDependency::builder("localstack")
        .with_impl(FakeLocalstackImpl { endpoint: None })
        .with_port(0)
        .with_image_tag("x")
        .build();

    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = dep.playbook();
    }));

    assert!(outcome.is_err());
}

#[tokio::test]
async fn run_empty_queues_active_drop_succeeds() {
    let mut dep = LocalstackDependency::builder("localstack")
        .with_impl(FakeLocalstackImpl { endpoint: None })
        .with_port(0)
        .with_image_tag("x")
        .with_readiness_check(OkReadinessCheck)
        .build();

    dep.start().await.expect("start should succeed");

    let active = dep
        .playbook()
        .with_identifier("session-purge")
        .run()
        .await;

    assert_eq!(active.identifier(), "session-purge");
}

async fn started_playbook_dep() -> LocalstackDependency {
    let mut dep = LocalstackDependency::builder("localstack")
        .with_impl(FakeLocalstackImpl { endpoint: None })
        .with_port(0)
        .with_image_tag("x")
        .with_readiness_check(OkReadinessCheck)
        .build();

    dep.start().await.expect("start should succeed");
    dep
}

#[tokio::test]
async fn active_playbook_endpoint_started_dep_returns_endpoint() {
    let dep = started_playbook_dep().await;

    let active = dep.playbook().run().await;

    assert_eq!(active.endpoint(), "http://127.0.0.1:4566");
}

#[tokio::test]
async fn active_playbook_queue_url_no_queues_returns_none() {
    let dep = started_playbook_dep().await;

    let active = dep.playbook().run().await;

    assert_eq!(active.queue_url("missing-queue"), None);
}

struct EmptyEndpointLocalstackImpl;

#[async_trait]
impl LocalstackImpl for EmptyEndpointLocalstackImpl {
    async fn start(
        &mut self,
        _port: u16,
        _image_name: &str,
        _image_tag: &str,
        _container_name: &str,
        _services: &[String],
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


    fn endpoint_url(&self) -> Option<&str> {
        Some("")
    }
}

#[tokio::test]
async fn active_playbook_drop_empty_endpoint_skips_cleanup_thread() {
    let mut dep = LocalstackDependency::builder("localstack")
        .with_impl(EmptyEndpointLocalstackImpl)
        .with_port(0)
        .with_image_tag("x")
        .with_readiness_check(OkReadinessCheck)
        .build();

    dep.start().await.expect("start should succeed");

    let active = dep.playbook().run().await;
    assert_eq!(active.endpoint(), "");

    drop(active);
}

#[tokio::test]
async fn active_playbook_drop_during_panic_skips_outcome_check() {
    let dep = started_playbook_dep().await;
    let active = dep.playbook().run().await;

    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
        let _active = active;
        panic!("simulated failure while active playbook is in scope");
    }));

    assert!(outcome.is_err());
}
