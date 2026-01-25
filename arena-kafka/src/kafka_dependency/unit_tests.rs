use super::*;
use async_trait::async_trait;
use std::sync::{Arc, Mutex};
use super::healthcheck::KafkaHealthcheckOps;

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

struct FakeOps {
    events: Arc<Mutex<Vec<Event>>>,
}

#[async_trait]
impl KafkaHealthcheckOps for FakeOps {
    async fn create_topic(&self, _bootstrap: &str, _topic: &str) -> Result<(), String> {
        self.events.lock().unwrap().push(Event::HealthcheckCreate);
        Ok(())
    }

    async fn delete_topic(&self, _bootstrap: &str, _topic: &str) -> Result<(), String> {
        self.events.lock().unwrap().push(Event::HealthcheckDelete);
        Ok(())
    }

    async fn publish(&self, _bootstrap: &str, _topic: &str, _payload: &str) -> Result<(), String> {
        self.events.lock().unwrap().push(Event::HealthcheckPublish);
        Ok(())
    }

    async fn consume_verify(
        &self,
        _bootstrap: &str,
        _topic: &str,
        _expected_payload: &str,
    ) -> Result<bool, String> {
        self.events.lock().unwrap().push(Event::HealthcheckConsume);
        Ok(true)
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

    let mut kafka = KafkaDependency {
        identifier: "kafka".to_string(),
        kafka_impl: Box::new(FakeKafkaImpl {
            bootstrap: Some("127.0.0.1:9092".to_string()),
            events: events.clone(),
        }),
        port: 0,
        dependencies: Some(deps),
        running: false,
        image_tag: "x".to_string(),
        container_name: Some("kafka-test".to_string()),
        healthcheck_ops: Box::new(FakeOps {
            events: events.clone(),
        }),
    };

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
            Event::KafkaStop,
            Event::DepStop("dep-b"),
            Event::DepStop("dep-a"),
        ]
    );
}

