use arena::lifecycle::{Fault, RunnableState};
use arena::dependency::{Dependency, RunnableDependency};
use arena::healthcheck::ReadinessCheck;
use arena_kafka::{KafkaDependency, KafkaImpl};
use async_trait::async_trait;
use futures::FutureExt;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq, Eq)]
enum Event {
    DepStart(&'static str),
    DepStop(&'static str),
    KafkaStart,
    KafkaStop,
    HealthcheckCreate,
    HealthcheckPublish,
    HealthcheckConsume,
    HealthcheckDelete,
    ReadinessCheck,
}

struct FakeKafkaImpl {
    bootstrap: Option<String>,
    events: Arc<Mutex<Vec<Event>>>,
}

#[async_trait]
impl KafkaImpl for FakeKafkaImpl {
    async fn start(
        &mut self,
        _port: u16,
        _image_name: &str,
        _image_tag: &str,
        _container_name: &str,
    ) -> Result<(), String> {
        self.bootstrap = Some("127.0.0.1:9092".to_string());
        self.events.lock().unwrap().push(Event::KafkaStart);
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), String> {
        self.events.lock().unwrap().push(Event::KafkaStop);
        Ok(())
    }
    async fn force_stop(&mut self) -> bool {
        true
    }
    fn release(&mut self) {}


    fn bootstrap_servers(&self) -> Option<&str> {
        self.bootstrap.as_deref()
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
        _bootstrap_servers: &str,
        _timeout_ms: u64,
    ) -> Result<(), String> {
        let mut ev = self.events.lock().unwrap();
        ev.push(Event::HealthcheckCreate);
        ev.push(Event::HealthcheckPublish);
        ev.push(Event::HealthcheckConsume);
        ev.push(Event::HealthcheckDelete);
        ev.push(Event::ReadinessCheck);
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

    async fn force_stop(&mut self) {}
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
        _bootstrap_servers: &str,
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

    let mut kafka = KafkaDependency::builder("kafka")
        .with_impl(FakeKafkaImpl {
            bootstrap: Some("127.0.0.1:9092".to_string()),
            events: events.clone(),
        })
        .with_port(0)
        .with_child_dependencies(deps)
        .with_image_tag("x")
        .with_readiness_check(FakeReadinessCheck {
            events: events.clone(),
        })
        .build();

    kafka.start().await.expect("start should succeed");
    kafka.stop().await.expect("stop should succeed");

    let got = events.lock().unwrap().clone();
    assert_eq!(
        got,
        vec![
            Event::DepStart("dep-a"),
            Event::DepStart("dep-b"),
            Event::KafkaStart,
            Event::HealthcheckCreate,
            Event::HealthcheckPublish,
            Event::HealthcheckConsume,
            Event::HealthcheckDelete,
            Event::ReadinessCheck,
            Event::KafkaStop,
            Event::DepStop("dep-b"),
            Event::DepStop("dep-a"),
        ]
    );
}

#[tokio::test]
async fn start_readiness_err_panics_after_impl_start() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let mut dep = KafkaDependency::builder("kafka")
        .with_impl(FakeKafkaImpl {
            bootstrap: None,
            events: events.clone(),
        })
        .with_port(0)
        .with_image_tag("x")
        .with_readiness_check(FailingReadinessCheck)
        .build();

    let outcome = std::panic::AssertUnwindSafe(async {
        dep.start().await.expect("start should succeed");
    })
    .catch_unwind()
    .await;

    assert!(outcome.is_err());
    assert_eq!(events.lock().unwrap().as_slice(), &[Event::KafkaStart]);
}

#[test]
fn identifier_as_any_and_children_reflect_dependency_state() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let mut dep = KafkaDependency::builder("kafka-accessors")
        .with_impl(FakeKafkaImpl {
            bootstrap: None,
            events: events.clone(),
        })
        .with_port(0)
        .with_image_tag("x")
        .build();

    assert!(dep.identifier().contains("kafka-accessors"));
    assert!(dep.as_any().downcast_ref::<KafkaDependency>().is_some());
    assert!(dep.as_any_mut().downcast_mut::<KafkaDependency>().is_some());
    assert!(dep.children().is_empty());

    dep.add_child(Box::new(FakeDep {
        name: "kafka-child",
        events: events.clone(),
        stopped: false,
    }));

    assert_eq!(dep.children().len(), 1);
    assert_eq!(dep.children_mut().len(), 1);
}

struct FlakyKafkaImpl {
    calls: AtomicUsize,
    ready_after: usize,
    bootstrap: String,
}

#[async_trait]
impl KafkaImpl for FlakyKafkaImpl {
    async fn start(&mut self, _port: u16, _image_name: &str, _image_tag: &str, _container_name: &str) -> Result<(), String> {
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), String> {
        Ok(())
    }
    async fn force_stop(&mut self) -> bool {
        true
    }
    fn release(&mut self) {}


    fn bootstrap_servers(&self) -> Option<&str> {
        let seen = self.calls.fetch_add(1, Ordering::SeqCst);
        if seen < self.ready_after {
            None
        } else {
            Some(&self.bootstrap)
        }
    }
}

struct ImmediateReadinessCheck;

#[async_trait]
impl ReadinessCheck for ImmediateReadinessCheck {
    async fn is_ready(&self, _identifier: &str, _bootstrap_servers: &str, _timeout_ms: u64) -> Result<(), String> {
        Ok(())
    }
}

#[tokio::test]
async fn wait_until_ready_retries_until_impl_reports_bootstrap() {
    let mut dep = KafkaDependency::builder("kafka-flaky")
        .with_impl(FlakyKafkaImpl {
            calls: AtomicUsize::new(0),
            ready_after: 2,
            bootstrap: "127.0.0.1:9092".to_string(),
        })
        .with_port(0)
        .with_image_tag("x")
        .with_readiness_check(ImmediateReadinessCheck)
        .build();

    dep.start().await.expect("start should succeed");

    assert_eq!(dep.bootstrap_servers(), Some("127.0.0.1:9092"));

    dep.stop().await.expect("stop should succeed");
}
