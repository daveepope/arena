use arena::lifecycle::{Fault, RunnableState};
use arena::dependency::{Dependency, RunnableDependency};
use arena::healthcheck::ReadinessCheck;
use arena_temporal::{TemporalDependency, TemporalImpl};
use async_trait::async_trait;
use futures::FutureExt;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq, Eq)]
enum Event {
    TemporalStart,
    TemporalStop,
    ReadinessCheck,
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
        true
    }
    fn release(&mut self) {}


    fn grpc_endpoint(&self) -> Option<&str> {
        self.grpc_endpoint.as_deref()
    }

    fn ui_url(&self) -> Option<&str> {
        self.ui_url.as_deref()
    }
}

struct FakeReadinessCheck {
    events: Arc<Mutex<Vec<Event>>>,
    last_identifier: Arc<Mutex<Option<String>>>,
    last_grpc_endpoint: Arc<Mutex<Option<String>>>,
    last_timeout_ms: Arc<Mutex<Option<u64>>>,
}

#[async_trait]
impl ReadinessCheck for FakeReadinessCheck {
    async fn is_ready(
        &self,
        identifier: &str,
        grpc_endpoint: &str,
        timeout_ms: u64,
    ) -> Result<(), String> {
        self.events.lock().unwrap().push(Event::ReadinessCheck);
        *self.last_identifier.lock().unwrap() = Some(identifier.to_string());
        *self.last_grpc_endpoint.lock().unwrap() = Some(grpc_endpoint.to_string());
        *self.last_timeout_ms.lock().unwrap() = Some(timeout_ms);
        Ok(())
    }
}

struct FailingReadinessCheck;

#[async_trait]
impl ReadinessCheck for FailingReadinessCheck {
    async fn is_ready(
        &self,
        _identifier: &str,
        _grpc_endpoint: &str,
        _timeout_ms: u64,
    ) -> Result<(), String> {
        Err("readiness failed".to_string())
    }
}

#[tokio::test]
async fn start_stop_happy_path_records_events() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let last_identifier = Arc::new(Mutex::new(None::<String>));
    let last_grpc_endpoint = Arc::new(Mutex::new(None::<String>));
    let last_timeout_ms = Arc::new(Mutex::new(None::<u64>));

    let mut temporal = TemporalDependency::builder("temporal")
        .with_impl(FakeTemporalImpl {
            grpc_endpoint: None,
            ui_url: None,
            events: events.clone(),
        })
        .with_readiness_check(FakeReadinessCheck {
            events: events.clone(),
            last_identifier: last_identifier.clone(),
            last_grpc_endpoint: last_grpc_endpoint.clone(),
            last_timeout_ms: last_timeout_ms.clone(),
        })
        .build();

    let outcome = std::panic::AssertUnwindSafe(async {
        temporal.start().await.expect("start should succeed");
        temporal.stop().await.expect("stop should succeed");
    })
    .catch_unwind()
    .await;

    assert!(outcome.is_ok(), "expected start/stop not to panic");

    let got = events.lock().unwrap().clone();
    assert_eq!(
        got,
        vec![
            Event::TemporalStart,
            Event::ReadinessCheck,
            Event::TemporalStop
        ]
    );

    assert_eq!(
        last_identifier.lock().unwrap().as_deref(),
        Some(temporal.identifier.as_str())
    );
    assert_eq!(
        last_grpc_endpoint.lock().unwrap().as_deref(),
        Some("127.0.0.1:7233")
    );
    let timeout_ms = last_timeout_ms
        .lock()
        .unwrap()
        .expect("readiness check should have been called with a timeout");
    assert!(
        timeout_ms <= 30_000,
        "expected timeout_ms <= 30_000, got {timeout_ms}"
    );
}

#[tokio::test]
async fn start_readiness_err_panics_after_impl_start() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let mut dep = TemporalDependency::builder("temporal")
        .with_impl(FakeTemporalImpl {
            grpc_endpoint: None,
            ui_url: None,
            events: events.clone(),
        })
        .with_readiness_check(FailingReadinessCheck)
        .build();

    let outcome = std::panic::AssertUnwindSafe(async {
        dep.start().await.expect("start should succeed");
    })
    .catch_unwind()
    .await;

    assert!(outcome.is_err());
    assert_eq!(events.lock().unwrap().as_slice(), &[Event::TemporalStart]);
}

struct NoopChildDependency;

#[async_trait]
impl RunnableDependency for NoopChildDependency {
    fn identifier(&self) -> &str {
        "temporal-child"
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

#[test]
fn identifier_as_any_and_children_reflect_dependency_state() {
    let mut dep = TemporalDependency::builder("temporal-accessors")
        .with_impl(FakeTemporalImpl {
            grpc_endpoint: None,
            ui_url: None,
            events: Arc::new(Mutex::new(Vec::new())),
        })
        .build();

    assert!(dep.identifier().contains("temporal-accessors"));
    assert!(dep.as_any().downcast_ref::<TemporalDependency>().is_some());
    assert!(dep.as_any_mut().downcast_mut::<TemporalDependency>().is_some());
    assert!(dep.children().is_empty());

    dep.add_child(Box::new(NoopChildDependency));

    assert_eq!(dep.children().len(), 1);
    assert_eq!(dep.children_mut().len(), 1);
}

struct FlakyTemporalImpl {
    calls: AtomicUsize,
    ready_after: usize,
    endpoint: String,
}

#[async_trait]
impl TemporalImpl for FlakyTemporalImpl {
    async fn start(
        &mut self,
        _grpc_port: u16,
        _ui_port: u16,
        _image_name: &str,
        _image_tag: &str,
        _container_name: &str,
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


    fn grpc_endpoint(&self) -> Option<&str> {
        let seen = self.calls.fetch_add(1, Ordering::SeqCst);
        if seen < self.ready_after {
            None
        } else {
            Some(&self.endpoint)
        }
    }

    fn ui_url(&self) -> Option<&str> {
        None
    }
}

struct ImmediateReadinessCheck;

#[async_trait]
impl ReadinessCheck for ImmediateReadinessCheck {
    async fn is_ready(&self, _identifier: &str, _grpc_endpoint: &str, _timeout_ms: u64) -> Result<(), String> {
        Ok(())
    }
}

#[tokio::test]
async fn wait_until_ready_retries_until_impl_reports_endpoint() {
    let mut dep = TemporalDependency::builder("temporal-flaky")
        .with_impl(FlakyTemporalImpl {
            calls: AtomicUsize::new(0),
            ready_after: 2,
            endpoint: "127.0.0.1:7233".to_string(),
        })
        .with_readiness_check(ImmediateReadinessCheck)
        .build();

    dep.start().await.expect("start should succeed");

    assert_eq!(dep.grpc_endpoint(), Some("127.0.0.1:7233"));

    dep.stop().await.expect("stop should succeed");
}
