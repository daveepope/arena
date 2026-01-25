use super::*;
use async_trait::async_trait;
use std::sync::{Arc, Mutex};
use arena::healthcheck::ReadinessCheck;

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
    async fn start(&mut self, _port: u16, _image_tag: &str, _container_name: &str) {
        self.events.lock().unwrap().push(Event::KafkaStart);
    }

    async fn stop(&mut self) {
        self.events.lock().unwrap().push(Event::KafkaStop);
    }

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
        _timeout: std::time::Duration,
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
}

#[tokio::test]
async fn kafka_dependency_lifecycle() {
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

    kafka.start().await;
    kafka.stop().await;

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

