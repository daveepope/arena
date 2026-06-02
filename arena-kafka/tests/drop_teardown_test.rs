use arena::dependency::RunnableDependency;
use arena::healthcheck::ReadinessCheck;
use arena_kafka::{KafkaDependency, KafkaImpl};
use async_trait::async_trait;
use futures::FutureExt;
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

struct OkReadinessCheck;

#[async_trait]
impl ReadinessCheck for OkReadinessCheck {
    async fn is_ready(
        &self,
        _identifier: &str,
        _bootstrap_servers: &str,
        _timeout_ms: u64,
    ) -> Result<(), String> {
        Ok(())
    }
}

struct PanickingKafkaReadinessCheck;

#[async_trait]
impl ReadinessCheck for PanickingKafkaReadinessCheck {
    async fn is_ready(
        &self,
        _identifier: &str,
        _bootstrap_servers: &str,
        _timeout_ms: u64,
    ) -> Result<(), String> {
        panic!("readiness probe failed");
    }
}

fn kafka_stop_count(events: &[Event]) -> usize {
    events
        .iter()
        .filter(|event| matches!(event, Event::KafkaStop))
        .count()
}

fn build_kafka(events: Arc<Mutex<Vec<Event>>>) -> KafkaDependency {
    KafkaDependency::builder("kafka-drop")
        .with_impl(FakeKafkaImpl {
            bootstrap: None,
            events,
        })
        .with_port(0)
        .with_image_tag("x")
        .with_readiness_check(OkReadinessCheck)
        .build()
}

#[test]
fn drop_unstarted_dep_skips_impl_stop() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let dep = build_kafka(events.clone());
    drop(dep);
    assert_eq!(kafka_stop_count(&events.lock().unwrap()), 0);
}

#[tokio::test]
async fn stop_then_drop_single_impl_stop() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let mut dep = build_kafka(events.clone());
    dep.start().await;
    dep.stop().await;
    drop(dep);
    assert_eq!(kafka_stop_count(&events.lock().unwrap()), 1);
}

#[tokio::test]
async fn drop_running_dep_invokes_full_stop() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let mut dep = build_kafka(events.clone());
    dep.start().await;
    drop(dep);
    assert_eq!(kafka_stop_count(&events.lock().unwrap()), 1);
}

#[tokio::test]
async fn start_panic_then_drop_impl_stop() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let mut dep = KafkaDependency::builder("kafka-drop")
        .with_impl(FakeKafkaImpl {
            bootstrap: None,
            events: events.clone(),
        })
        .with_port(0)
        .with_image_tag("x")
        .with_readiness_check(PanickingKafkaReadinessCheck)
        .build();

    let start_outcome = std::panic::AssertUnwindSafe(async {
        dep.start().await;
    })
    .catch_unwind()
    .await;
    assert!(start_outcome.is_err());
    assert_eq!(events.lock().unwrap().as_slice(), &[Event::KafkaStart]);

    drop(dep);
    assert_eq!(kafka_stop_count(&events.lock().unwrap()), 1);
}
