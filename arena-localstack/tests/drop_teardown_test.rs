use arena::dependency::RunnableDependency;
use arena::lifecycle::RunnableState;
use arena::healthcheck::ReadinessCheck;
use arena_localstack::{LocalstackDependency, LocalstackImpl};
use async_trait::async_trait;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq, Eq)]
enum Event {
    LocalstackStart,
    LocalstackStop,
    LocalstackForceStop,
    LocalstackRelease,
}

struct FakeLocalstackImpl {
    endpoint: Option<String>,
    events: Arc<Mutex<Vec<Event>>>,
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
        self.events.lock().unwrap().push(Event::LocalstackStart);
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), String> {
        self.events.lock().unwrap().push(Event::LocalstackStop);
        Ok(())
    }
    async fn force_stop(&mut self) -> bool {
        self.events.lock().unwrap().push(Event::LocalstackForceStop);
        true
    }
    fn release(&mut self) {
        self.events.lock().unwrap().push(Event::LocalstackRelease);
    }


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

struct FailingLocalstackReadinessCheck;

#[async_trait]
impl ReadinessCheck for FailingLocalstackReadinessCheck {
    async fn is_ready(
        &self,
        _identifier: &str,
        _endpoint: &str,
        _timeout_ms: u64,
    ) -> Result<(), String> {
        Err("readiness probe failed".to_string())
    }
}

fn localstack_stop_count(events: &[Event]) -> usize {
    events
        .iter()
        .filter(|event| matches!(event, Event::LocalstackStop))
        .count()
}

fn force_stop_count(events: &[Event]) -> usize {
    events
        .iter()
        .filter(|event| matches!(event, Event::LocalstackForceStop))
        .count()
}

fn release_count(events: &[Event]) -> usize {
    events
        .iter()
        .filter(|event| matches!(event, Event::LocalstackRelease))
        .count()
}

fn build_localstack(events: Arc<Mutex<Vec<Event>>>) -> LocalstackDependency {
    LocalstackDependency::builder("localstack-drop")
        .with_impl(FakeLocalstackImpl {
            endpoint: None,
            events,
        })
        .with_port(0)
        .with_image_tag("x")
        .with_readiness_check(OkReadinessCheck)
        .build()
}

fn build_localstack_with_failing_readiness(events: Arc<Mutex<Vec<Event>>>) -> LocalstackDependency {
    LocalstackDependency::builder("localstack-drop")
        .with_impl(FakeLocalstackImpl {
            endpoint: None,
            events,
        })
        .with_port(0)
        .with_image_tag("x")
        .with_readiness_check(FailingLocalstackReadinessCheck)
        .build()
}

#[test]
fn drop_unstarted_dep_skips_impl_stop() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let dep = build_localstack(events.clone());
    drop(dep);
    assert_eq!(localstack_stop_count(&events.lock().unwrap()), 0);
}

#[tokio::test]
async fn stop_then_drop_single_impl_stop() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let mut dep = build_localstack(events.clone());
    dep.start().await.expect("start should succeed");
    dep.stop().await.expect("stop should succeed");
    drop(dep);
    assert_eq!(localstack_stop_count(&events.lock().unwrap()), 1);
}

#[tokio::test]
async fn drop_running_dependency_releases_container() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let mut dep = build_localstack(events.clone());
    dep.start().await.expect("start should succeed");
    drop(dep);
    assert_eq!(release_count(&events.lock().unwrap()), 1);
}

#[tokio::test]
async fn start_readiness_failure_returns_fault_and_forces_stop() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let mut dep = build_localstack_with_failing_readiness(events.clone());

    let fault = dep.start().await.expect_err("dependency should fault");

    assert_eq!(fault.id, dep.identifier());
    assert_eq!(dep.state(), RunnableState::Stopped);
    assert_eq!(dep.faults().len(), 1);
    assert_eq!(force_stop_count(&events.lock().unwrap()), 1);
}

#[tokio::test]
async fn start_readiness_failure_then_drop_does_not_force_stop_twice() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let mut dep = build_localstack_with_failing_readiness(events.clone());

    let _fault = dep.start().await.expect_err("dependency should fault");
    drop(dep);

    assert_eq!(force_stop_count(&events.lock().unwrap()), 1);
}

#[tokio::test]
async fn force_stop_called_twice_is_indistinguishable_from_once() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let mut dep = build_localstack(events.clone());

    dep.start().await.expect("dependency should start");
    dep.force_stop().await;
    let after_first = dep.state();
    dep.force_stop().await;

    assert_eq!(after_first, RunnableState::Stopped);
    assert_eq!(dep.state(), RunnableState::Stopped);
    assert!(dep.faults().is_empty());
}
