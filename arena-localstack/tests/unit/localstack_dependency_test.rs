use arena::dependency::{Dependency, RunnableDependency};
use arena::healthcheck::ReadinessCheck;
use arena::lifecycle::{Fault, RunnableState};
use arena_localstack::{
    EventRuleSpec, EventRuleTarget, EventTargetKind, LocalstackDependency, LocalstackImpl,
};
use async_trait::async_trait;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

const CLOSED_PORT_ENDPOINT: &str = "http://127.0.0.1:1";

struct FakeLocalstackImpl {
    endpoint_url: Option<String>,
    force_stop_confirms_removal: bool,
    releases: Arc<AtomicUsize>,
}

impl FakeLocalstackImpl {
    fn new() -> Self {
        Self {
            endpoint_url: None,
            force_stop_confirms_removal: true,
            releases: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn without_confirmed_removal(mut self) -> Self {
        self.force_stop_confirms_removal = false;
        self
    }
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
        self.endpoint_url = Some(CLOSED_PORT_ENDPOINT.to_string());
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), String> {
        self.endpoint_url = None;
        Ok(())
    }

    async fn force_stop(&mut self) -> bool {
        self.release();
        self.force_stop_confirms_removal
    }

    fn release(&mut self) {
        self.endpoint_url = None;
        self.releases.fetch_add(1, Ordering::SeqCst);
    }

    fn endpoint_url(&self) -> Option<&str> {
        self.endpoint_url.as_deref()
    }
}

struct PassingReadinessCheck;

#[async_trait]
impl ReadinessCheck for PassingReadinessCheck {
    async fn is_ready(&self, _: &str, _: &str, _: u64) -> Result<(), String> {
        Ok(())
    }
}

#[derive(Default)]
struct ChildCalls {
    released: usize,
    force_stopped: usize,
}

struct FakeChildDependency {
    calls: Arc<Mutex<ChildCalls>>,
}

#[async_trait]
impl RunnableDependency for FakeChildDependency {
    fn identifier(&self) -> &str {
        "localstack-child"
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

    async fn start(&mut self) -> Result<(), Fault> {
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), Fault> {
        Ok(())
    }

    async fn force_stop(&mut self) {
        self.calls.lock().unwrap().force_stopped += 1;
    }

    fn release(&mut self) {
        self.calls.lock().unwrap().released += 1;
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

fn setup_dependency(identifier: &str, localstack_impl: FakeLocalstackImpl) -> LocalstackDependency {
    LocalstackDependency::builder(identifier)
        .with_impl(localstack_impl)
        .with_readiness_check(PassingReadinessCheck)
        .build()
}

fn setup_dependency_with_rule(identifier: &str, target: EventTargetKind) -> LocalstackDependency {
    LocalstackDependency::builder(identifier)
        .with_impl(FakeLocalstackImpl::new())
        .with_readiness_check(PassingReadinessCheck)
        .with_event_rule(EventRuleSpec {
            name: "orders-rule".to_string(),
            event_bus: None,
            event_pattern: "{}".to_string(),
            targets: vec![EventRuleTarget {
                target_id: "target-1".to_string(),
                kind: target,
            }],
        })
        .build()
}

#[tokio::test]
async fn release_started_dependency_releases_container_and_children() {
    let localstack_impl = FakeLocalstackImpl::new();
    let releases = localstack_impl.releases.clone();
    let calls = Arc::new(Mutex::new(ChildCalls::default()));
    let mut dep = setup_dependency("localstack-release", localstack_impl);
    dep.add_child(Box::new(FakeChildDependency {
        calls: calls.clone(),
    }));
    dep.start().await.expect("start should succeed");

    dep.release();

    assert_eq!(dep.state(), RunnableState::Stopped);
    assert_eq!(releases.load(Ordering::SeqCst), 1);
    assert_eq!(calls.lock().unwrap().released, 1);
}

#[tokio::test]
async fn force_stop_repeated_unconfirmed_removal_records_one_fault() {
    let calls = Arc::new(Mutex::new(ChildCalls::default()));
    let mut dep = setup_dependency(
        "localstack-force-stop",
        FakeLocalstackImpl::new().without_confirmed_removal(),
    );
    dep.add_child(Box::new(FakeChildDependency {
        calls: calls.clone(),
    }));

    dep.force_stop().await;
    dep.force_stop().await;

    assert_eq!(dep.state(), RunnableState::Faulted);
    assert_eq!(dep.faults().len(), 1);
    assert_eq!(calls.lock().unwrap().force_stopped, 2);
}

#[tokio::test]
async fn start_event_rule_targeting_unknown_queue_returns_fault() {
    let mut dep = setup_dependency_with_rule(
        "localstack-unknown-queue",
        EventTargetKind::SqsQueue {
            queue_name: "missing-queue".to_string(),
        },
    );

    let fault = dep.start().await.expect_err("start should fault");

    assert!(
        fault.message.contains("unknown queue missing-queue"),
        "unexpected fault: {}",
        fault.message
    );
    assert_eq!(dep.faults().len(), 1);
    assert_eq!(dep.state(), RunnableState::Stopped);
}

#[tokio::test]
async fn start_event_rule_targeting_unknown_lambda_returns_fault() {
    let mut dep = setup_dependency_with_rule(
        "localstack-unknown-lambda",
        EventTargetKind::Lambda {
            function_name: "missing-lambda".to_string(),
        },
    );

    let fault = dep.start().await.expect_err("start should fault");

    assert!(
        fault.message.contains("unknown lambda missing-lambda"),
        "unexpected fault: {}",
        fault.message
    );
    assert_eq!(dep.faults().len(), 1);
    assert_eq!(dep.state(), RunnableState::Stopped);
}
