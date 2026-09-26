use arena::dependency::RunnableDependency;
use arena::healthcheck::ReadinessCheck;
use arena::lifecycle::RunnableState;
use arena_temporal::{TemporalDependency, TemporalImpl};
use async_trait::async_trait;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq, Eq)]
enum Event {
    TemporalStart,
    TemporalStop,
    TemporalForceStop,
    TemporalRelease,
}

struct FakeTemporalImpl {
    grpc_endpoint: Option<String>,
    ui_url: Option<String>,
    events: Arc<Mutex<Vec<Event>>>,
}

#[async_trait]
impl TemporalImpl for FakeTemporalImpl {
    async fn start(
        &mut self,
        _grpc_port: u16,
        _ui_port: u16,
        _image_name: &str,
        _image_tag: &str,
        _container_name: &str,
    ) -> Result<(), String> {
        self.grpc_endpoint = Some("127.0.0.1:7233".to_string());
        self.ui_url = Some("http://127.0.0.1:8233".to_string());
        self.events.lock().unwrap().push(Event::TemporalStart);
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), String> {
        self.grpc_endpoint = None;
        self.ui_url = None;
        self.events.lock().unwrap().push(Event::TemporalStop);
        Ok(())
    }

    async fn force_stop(&mut self) -> bool {
        self.grpc_endpoint = None;
        self.ui_url = None;
        self.events.lock().unwrap().push(Event::TemporalForceStop);
        true
    }
    fn release(&mut self) {
        self.events.lock().unwrap().push(Event::TemporalRelease);
    }


    fn grpc_endpoint(&self) -> Option<&str> {
        self.grpc_endpoint.as_deref()
    }

    fn ui_url(&self) -> Option<&str> {
        self.ui_url.as_deref()
    }
}

struct OkReadinessCheck;

#[async_trait]
impl ReadinessCheck for OkReadinessCheck {
    async fn is_ready(
        &self,
        _identifier: &str,
        _grpc_endpoint: &str,
        _timeout_ms: u64,
    ) -> Result<(), String> {
        Ok(())
    }
}

struct FailingTemporalReadinessCheck;

#[async_trait]
impl ReadinessCheck for FailingTemporalReadinessCheck {
    async fn is_ready(
        &self,
        _identifier: &str,
        _grpc_endpoint: &str,
        _timeout_ms: u64,
    ) -> Result<(), String> {
        Err("readiness probe failed".to_string())
    }
}

fn count_of(events: &[Event], wanted: Event) -> usize {
    events.iter().filter(|event| **event == wanted).count()
}

fn build_temporal(events: Arc<Mutex<Vec<Event>>>) -> TemporalDependency {
    TemporalDependency::builder("temporal-drop")
        .with_impl(FakeTemporalImpl {
            grpc_endpoint: None,
            ui_url: None,
            events,
        })
        .with_readiness_check(OkReadinessCheck)
        .build()
}

fn build_temporal_with_failing_readiness(
    events: Arc<Mutex<Vec<Event>>>,
) -> TemporalDependency {
    TemporalDependency::builder("temporal-drop")
        .with_impl(FakeTemporalImpl {
            grpc_endpoint: None,
            ui_url: None,
            events,
        })
        .with_readiness_check(FailingTemporalReadinessCheck)
        .build()
}

#[test]
fn drop_unstarted_dependency_skips_teardown() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let dep = build_temporal(events.clone());

    drop(dep);

    assert!(events.lock().unwrap().is_empty());
}

#[tokio::test]
async fn state_unstarted_dependency_returns_not_started() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let dep = build_temporal(events);

    assert_eq!(dep.state(), RunnableState::NotStarted);
    assert!(dep.faults().is_empty());
}

#[tokio::test]
async fn start_healthy_dependency_returns_started() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let mut dep = build_temporal(events.clone());

    dep.start().await.expect("dependency should start");

    assert_eq!(dep.state(), RunnableState::Started);
    assert_eq!(count_of(&events.lock().unwrap(), Event::TemporalStart), 1);
}

#[tokio::test]
async fn stop_started_dependency_returns_stopped() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let mut dep = build_temporal(events.clone());

    dep.start().await.expect("dependency should start");
    dep.stop().await.expect("dependency should stop");

    assert_eq!(dep.state(), RunnableState::Stopped);
    assert_eq!(count_of(&events.lock().unwrap(), Event::TemporalStop), 1);
}

#[tokio::test]
async fn stop_then_drop_does_not_stop_twice() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let mut dep = build_temporal(events.clone());

    dep.start().await.expect("dependency should start");
    dep.stop().await.expect("dependency should stop");
    drop(dep);

    assert_eq!(count_of(&events.lock().unwrap(), Event::TemporalStop), 1);
    assert_eq!(count_of(&events.lock().unwrap(), Event::TemporalForceStop), 0);
}

#[tokio::test]
async fn drop_running_dependency_releases_container() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let mut dep = build_temporal(events.clone());

    dep.start().await.expect("dependency should start");
    drop(dep);

    assert_eq!(count_of(&events.lock().unwrap(), Event::TemporalRelease), 1);
}

#[tokio::test]
async fn start_readiness_failure_returns_fault_and_forces_stop() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let mut dep = build_temporal_with_failing_readiness(events.clone());

    let fault = dep.start().await.expect_err("dependency should fault");

    assert_eq!(fault.id, dep.identifier());
    assert!(fault.message.contains("readiness check failed"));
    assert_eq!(dep.state(), RunnableState::Stopped);
    assert_eq!(dep.faults().len(), 1);
    assert_eq!(count_of(&events.lock().unwrap(), Event::TemporalForceStop), 1);
}

#[tokio::test]
async fn start_readiness_failure_then_drop_does_not_force_stop_twice() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let mut dep = build_temporal_with_failing_readiness(events.clone());

    let _fault = dep.start().await.expect_err("dependency should fault");
    drop(dep);

    assert_eq!(count_of(&events.lock().unwrap(), Event::TemporalForceStop), 1);
}

#[tokio::test]
async fn force_stop_called_twice_is_indistinguishable_from_once() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let mut dep = build_temporal(events.clone());

    dep.start().await.expect("dependency should start");
    dep.force_stop().await;
    let after_first = dep.state();
    dep.force_stop().await;

    assert_eq!(after_first, RunnableState::Stopped);
    assert_eq!(dep.state(), RunnableState::Stopped);
    assert!(dep.faults().is_empty());
}
