use arena::lifecycle::{Fault, RunnableState};
use arena::dependency::{Dependency, RunnableDependency};
use arena::healthcheck::ReadinessCheck;
use arena_localstack::{LocalstackDependency, LocalstackImpl};
use async_trait::async_trait;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq, Eq)]
enum Event {
    DepStart(&'static str),
    DepStop(&'static str),
    DepForceStop(&'static str),
    LocalstackStart,
    LocalstackStop,
    LocalstackForceStop,
    ReadinessCheck,
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
        self.events
            .lock()
            .unwrap()
            .push(Event::LocalstackForceStop);
        true
    }
    fn release(&mut self) {}


    fn endpoint_url(&self) -> Option<&str> {
        self.endpoint.as_deref()
    }
}

struct FakeReadinessCheck {
    events: Arc<Mutex<Vec<Event>>>,
}

#[async_trait]
impl ReadinessCheck for FakeReadinessCheck {
    async fn is_ready(
        &self,
        _identifier: &str,
        _endpoint: &str,
        _timeout_ms: u64,
    ) -> Result<(), String> {
        self.events.lock().unwrap().push(Event::ReadinessCheck);
        Ok(())
    }
}

struct FakeDep {
    name: &'static str,
    events: Arc<Mutex<Vec<Event>>>,
    stopped: bool,
}

#[async_trait]
impl RunnableDependency for FakeDep {
    fn identifier(&self) -> &str {
        self.name
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

    async fn force_stop(&mut self) {
        self.events
            .lock()
            .unwrap()
            .push(Event::DepForceStop(self.name));
    }
    fn release(&mut self) {}


    async fn start(&mut self) -> Result<(), Fault> {
        self.events.lock().unwrap().push(Event::DepStart(self.name));
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), Fault> {
        if self.stopped {
            return Ok(());
        }
        self.events.lock().unwrap().push(Event::DepStop(self.name));
        self.stopped = true;
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

struct FailingReadinessCheck;

#[async_trait]
impl ReadinessCheck for FailingReadinessCheck {
    async fn is_ready(
        &self,
        _identifier: &str,
        _endpoint: &str,
        _timeout_ms: u64,
    ) -> Result<(), String> {
        Err("readiness failed".to_string())
    }
}

#[tokio::test]
async fn start_stop_happy_path_records_events() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));

    let deps: Vec<Box<dyn RunnableDependency>> = vec![
        Box::new(FakeDep {
            name: "dep-a",
            events: events.clone(),
            stopped: false,
        }),
        Box::new(FakeDep {
            name: "dep-b",
            events: events.clone(),
            stopped: false,
        }),
    ];

    let mut localstack = LocalstackDependency::builder("localstack")
        .with_impl(FakeLocalstackImpl {
            endpoint: Some("http://127.0.0.1:4566".to_string()),
            events: events.clone(),
        })
        .with_port(0)
        .with_child_dependencies(deps)
        .with_image_tag("x")
        .with_readiness_check(FakeReadinessCheck {
            events: events.clone(),
        })
        .build();

    localstack.start().await.expect("start should succeed");
    localstack.stop().await.expect("stop should succeed");

    let got = events.lock().unwrap().clone();
    assert_eq!(
        got,
        vec![
            Event::DepStart("dep-a"),
            Event::DepStart("dep-b"),
            Event::LocalstackStart,
            Event::ReadinessCheck,
            Event::LocalstackStop,
            Event::DepStop("dep-b"),
            Event::DepStop("dep-a"),
        ]
    );
}

#[tokio::test]
async fn start_readiness_failure_returns_fault_after_impl_start() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let mut dep = LocalstackDependency::builder("localstack")
        .with_impl(FakeLocalstackImpl {
            endpoint: None,
            events: events.clone(),
        })
        .with_port(0)
        .with_image_tag("x")
        .with_readiness_check(FailingReadinessCheck)
        .build();

    let fault = dep.start().await.expect_err("dependency should fault");

    assert!(fault.message.contains("readiness check failed"));
    assert_eq!(
        events.lock().unwrap().as_slice(),
        &[Event::LocalstackStart, Event::LocalstackForceStop]
    );
}

#[tokio::test]
async fn start_readiness_failure_then_stop_does_not_stop_children_twice() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));

    let deps: Vec<Box<dyn RunnableDependency>> = vec![Box::new(FakeDep {
        name: "dep-a",
        events: events.clone(),
        stopped: false,
    })];

    let mut dep = LocalstackDependency::builder("localstack")
        .with_impl(FakeLocalstackImpl {
            endpoint: Some("http://127.0.0.1:4566".to_string()),
            events: events.clone(),
        })
        .with_port(0)
        .with_child_dependencies(deps)
        .with_image_tag("x")
        .with_readiness_check(FailingReadinessCheck)
        .build();

    let _fault = dep.start().await.expect_err("dependency should fault");
    dep.stop().await.expect("stop should succeed");

    assert_eq!(
        events.lock().unwrap().as_slice(),
        &[
            Event::DepStart("dep-a"),
            Event::LocalstackStart,
            Event::LocalstackForceStop,
            Event::DepForceStop("dep-a"),
            Event::LocalstackStop,
            Event::DepStop("dep-a"),
        ]
    );
}

#[tokio::test]
async fn start_readiness_failure_returns_fault_and_force_stops_children() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));

    let deps: Vec<Box<dyn RunnableDependency>> = vec![Box::new(FakeDep {
        name: "dep-a",
        events: events.clone(),
        stopped: false,
    })];

    let mut dep = LocalstackDependency::builder("localstack")
        .with_impl(FakeLocalstackImpl {
            endpoint: Some("http://127.0.0.1:4566".to_string()),
            events: events.clone(),
        })
        .with_port(0)
        .with_child_dependencies(deps)
        .with_image_tag("x")
        .with_readiness_check(FailingReadinessCheck)
        .build();

    let fault = dep.start().await.expect_err("dependency should fault");

    assert_eq!(fault.id, dep.identifier());
    assert_eq!(dep.state(), RunnableState::Stopped);

    let observed = events.lock().unwrap().clone();
    drop(dep);

    assert_eq!(
        observed,
        &[
            Event::DepStart("dep-a"),
            Event::LocalstackStart,
            Event::LocalstackForceStop,
            Event::DepForceStop("dep-a"),
        ]
    );
    assert_eq!(
        events.lock().unwrap().as_slice(),
        observed,
        "a faulted dependency must not be torn down again on drop"
    );
}

#[test]
fn identifier_as_any_and_children_reflect_dependency_state() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let mut dep = LocalstackDependency::builder("localstack-accessors")
        .with_impl(FakeLocalstackImpl {
            endpoint: None,
            events: events.clone(),
        })
        .with_port(0)
        .with_image_tag("x")
        .build();

    assert!(dep.identifier().contains("localstack-accessors"));
    assert!(dep.as_any().downcast_ref::<LocalstackDependency>().is_some());
    assert!(dep
        .as_any_mut()
        .downcast_mut::<LocalstackDependency>()
        .is_some());
    assert!(dep.children().is_empty());
    assert_eq!(dep.queue_url("missing"), None);
    assert_eq!(dep.queue_arn("missing"), None);
    assert_eq!(dep.lambda_arn("missing"), None);
    assert!(dep.queue_urls_snapshot().is_empty());

    dep.add_child(Box::new(FakeDep {
        name: "localstack-child",
        events: events.clone(),
        stopped: false,
    }));

    assert_eq!(dep.children().len(), 1);
    assert_eq!(dep.children_mut().len(), 1);
}

struct FlakyLocalstackImpl {
    calls: AtomicUsize,
    ready_after: usize,
    endpoint: String,
}

#[async_trait]
impl LocalstackImpl for FlakyLocalstackImpl {
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
        let seen = self.calls.fetch_add(1, Ordering::SeqCst);
        if seen < self.ready_after {
            None
        } else {
            Some(&self.endpoint)
        }
    }
}

struct ImmediateReadinessCheck;

#[async_trait]
impl ReadinessCheck for ImmediateReadinessCheck {
    async fn is_ready(&self, _identifier: &str, _endpoint: &str, _timeout_ms: u64) -> Result<(), String> {
        Ok(())
    }
}

#[tokio::test]
async fn wait_until_ready_retries_until_impl_reports_endpoint() {
    let mut dep = LocalstackDependency::builder("localstack-flaky")
        .with_impl(FlakyLocalstackImpl {
            calls: AtomicUsize::new(0),
            ready_after: 2,
            endpoint: "http://127.0.0.1:4566".to_string(),
        })
        .with_port(0)
        .with_image_tag("x")
        .with_readiness_check(ImmediateReadinessCheck)
        .build();

    dep.start().await.expect("start should succeed");

    assert_eq!(dep.endpoint_url(), Some("http://127.0.0.1:4566"));

    dep.stop().await.expect("stop should succeed");
}
