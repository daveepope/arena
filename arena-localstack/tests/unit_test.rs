use arena::dependency::RunnableDependency;
use arena::healthcheck::ReadinessCheck;
use arena_localstack::{LocalstackDependency, LocalstackImpl};
use async_trait::async_trait;
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

    async fn soft_reset(&self) {}

    async fn hard_reset(&mut self) {}
}

#[tokio::test]
async fn localstack_dependency_lifecycle() {
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
