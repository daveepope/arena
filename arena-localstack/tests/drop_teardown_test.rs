use arena::dependency::RunnableDependency;
use arena::healthcheck::ReadinessCheck;
use arena_localstack::{LocalstackDependency, LocalstackImpl};
use async_trait::async_trait;
use futures::FutureExt;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq, Eq)]
enum Event {
    LocalstackStart,
    LocalstackStop,
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

struct OkReadinessCheck;

#[async_trait]
impl ReadinessCheck for OkReadinessCheck {
    async fn is_ready(
        &self,
        _identifier: &str,
        _endpoint: &str,
        _timeout_ms: u64,
    ) -> Result<(), String> {
        Ok(())
    }
}

struct PanickingLocalstackReadinessCheck;

#[async_trait]
impl ReadinessCheck for PanickingLocalstackReadinessCheck {
    async fn is_ready(
        &self,
        _identifier: &str,
        _endpoint: &str,
        _timeout_ms: u64,
    ) -> Result<(), String> {
        panic!("readiness probe failed");
    }
}

fn localstack_stop_count(events: &[Event]) -> usize {
    events
        .iter()
        .filter(|event| matches!(event, Event::LocalstackStop))
        .count()
}

fn build_localstack(events: Arc<Mutex<Vec<Event>>>) -> LocalstackDependency {
    LocalstackDependency::builder("localstack-drop")
        .with_impl(FakeLocalstackImpl {
            endpoint: None,
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
    let dep = build_localstack(events.clone());
    drop(dep);
    assert_eq!(localstack_stop_count(&events.lock().unwrap()), 0);
}

#[tokio::test]
async fn stop_then_drop_single_impl_stop() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let mut dep = build_localstack(events.clone());
    dep.start().await;
    dep.stop().await;
    drop(dep);
    assert_eq!(localstack_stop_count(&events.lock().unwrap()), 1);
}

#[tokio::test]
async fn drop_running_dep_invokes_full_stop() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let mut dep = build_localstack(events.clone());
    dep.start().await;
    drop(dep);
    assert_eq!(localstack_stop_count(&events.lock().unwrap()), 1);
}

#[tokio::test]
async fn start_panic_then_drop_impl_stop() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let mut dep = LocalstackDependency::builder("localstack-drop")
        .with_impl(FakeLocalstackImpl {
            endpoint: None,
            events: events.clone(),
        })
        .with_port(0)
        .with_image_tag("x")
        .with_readiness_check(PanickingLocalstackReadinessCheck)
        .build();

    let start_outcome = std::panic::AssertUnwindSafe(async {
        dep.start().await;
    })
    .catch_unwind()
    .await;
    assert!(start_outcome.is_err());
    assert_eq!(
        events.lock().unwrap().as_slice(),
        &[Event::LocalstackStart]
    );

    drop(dep);
    assert_eq!(localstack_stop_count(&events.lock().unwrap()), 1);
}
