use arena::dependency::RunnableDependency;
use arena::healthcheck::ReadinessCheck;
use arena_kafka::{KafkaDependency, KafkaImpl};
use async_trait::async_trait;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq, Eq)]
enum Event {
    KafkaStart,
    KafkaStop,
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
    ) {
        self.bootstrap = Some("127.0.0.1:9092".to_string());
        self.events.lock().unwrap().push(Event::KafkaStart);
    }

    async fn stop(&mut self) {
        self.events.lock().unwrap().push(Event::KafkaStop);
    }

    fn bootstrap_servers(&self) -> Option<&str> {
        self.bootstrap.as_deref()
    }
}

struct OkReadinessCheck {
    calls: Arc<AtomicUsize>,
}

#[async_trait]
impl ReadinessCheck for OkReadinessCheck {
    async fn is_ready(
        &self,
        _identifier: &str,
        _bootstrap_servers: &str,
        _timeout_ms: u64,
    ) -> Result<(), String> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

fn build_kafka(
    events: Arc<Mutex<Vec<Event>>>,
    readiness_calls: Arc<AtomicUsize>,
) -> KafkaDependency {
    KafkaDependency::builder("kafka-reset")
        .with_impl(FakeKafkaImpl {
            bootstrap: None,
            events,
        })
        .with_port(0)
        .with_image_tag("x")
        .with_readiness_check(OkReadinessCheck {
            calls: readiness_calls,
        })
        .build()
}

#[tokio::test]
async fn soft_reset_not_running_is_noop() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let readiness_calls = Arc::new(AtomicUsize::new(0));
    let dep = build_kafka(events.clone(), readiness_calls);

    dep.soft_reset().await;

    assert!(events.lock().unwrap().is_empty());
}

#[tokio::test]
async fn hard_reset_not_running_is_noop() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let readiness_calls = Arc::new(AtomicUsize::new(0));
    let mut dep = build_kafka(events.clone(), readiness_calls);

    dep.hard_reset().await;

    assert!(events.lock().unwrap().is_empty());
}

#[tokio::test]
async fn hard_reset_running_dep_restarts_impl() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let readiness_calls = Arc::new(AtomicUsize::new(0));
    let mut dep = build_kafka(events.clone(), readiness_calls.clone());

    dep.start().await;
    assert_eq!(readiness_calls.load(Ordering::SeqCst), 1);

    dep.hard_reset().await;

    assert_eq!(readiness_calls.load(Ordering::SeqCst), 2);
    assert_eq!(
        events.lock().unwrap().as_slice(),
        &[Event::KafkaStart, Event::KafkaStop, Event::KafkaStart]
    );

    dep.stop().await;
}

#[tokio::test]
async fn soft_reset_running_dep_with_no_topics_is_noop() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let readiness_calls = Arc::new(AtomicUsize::new(0));
    let mut dep = build_kafka(events.clone(), readiness_calls);

    dep.start().await;
    dep.soft_reset().await;

    dep.stop().await;
}

