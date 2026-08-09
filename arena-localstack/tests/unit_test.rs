use arena::dependency::{Dependency, RunnableDependency};
use arena::healthcheck::ReadinessCheck;
use arena_localstack::{LocalstackDependency, LocalstackImpl};
use async_trait::async_trait;
use futures::FutureExt;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq, Eq)]
enum Event {
    DepStart(&'static str),
    DepStop(&'static str),
    LocalstackStart,
    LocalstackStop,
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
    ) {
        self.endpoint = Some("http://127.0.0.1:4566".to_string());
        self.events.lock().unwrap().push(Event::LocalstackStart);
    }

    async fn stop(&mut self) {
        self.events.lock().unwrap().push(Event::LocalstackStop);
    }

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

    async fn start(&mut self) {
        self.events.lock().unwrap().push(Event::DepStart(self.name));
    }

    async fn stop(&mut self) {
        if self.stopped {
            return;
        }
        self.events.lock().unwrap().push(Event::DepStop(self.name));
        self.stopped = true;
    }

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

    localstack.start().await;
    localstack.stop().await;

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
async fn start_readiness_err_panics_after_impl_start() {
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

    let outcome = std::panic::AssertUnwindSafe(async {
        dep.start().await;
    })
    .catch_unwind()
    .await;

    assert!(outcome.is_err());
    assert_eq!(
        events.lock().unwrap().as_slice(),
        &[Event::LocalstackStart]
    );
}

#[tokio::test]
async fn start_readiness_err_stop_stops_started_children() {
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

    let outcome = std::panic::AssertUnwindSafe(async {
        dep.start().await;
    })
    .catch_unwind()
    .await;

    assert!(outcome.is_err());
    dep.stop().await;

    assert_eq!(
        events.lock().unwrap().as_slice(),
        &[
            Event::DepStart("dep-a"),
            Event::LocalstackStart,
            Event::LocalstackStop,
            Event::DepStop("dep-a"),
        ]
    );
}

#[tokio::test]
async fn start_readiness_err_drop_stops_started_children() {
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

    let outcome = std::panic::AssertUnwindSafe(async {
        dep.start().await;
    })
    .catch_unwind()
    .await;

    assert!(outcome.is_err());
    drop(dep);

    assert_eq!(
        events.lock().unwrap().as_slice(),
        &[
            Event::DepStart("dep-a"),
            Event::LocalstackStart,
            Event::LocalstackStop,
            Event::DepStop("dep-a"),
        ]
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
    ) {
    }

    async fn stop(&mut self) {}

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

    dep.start().await;

    assert_eq!(dep.endpoint_url(), Some("http://127.0.0.1:4566"));

    dep.stop().await;
}
