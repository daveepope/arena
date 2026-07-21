use arena::dependency::RunnableDependency;
use arena::healthcheck::ReadinessCheck;
use arena_temporal::{TemporalDependency, TemporalImpl};
use async_trait::async_trait;
use futures::FutureExt;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq, Eq)]
enum Event {
    TemporalStart,
    TemporalStop,
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
    ) {
        self.grpc_endpoint = Some("127.0.0.1:7233".to_string());
        self.ui_url = Some("http://127.0.0.1:8233".to_string());
        self.events.lock().unwrap().push(Event::TemporalStart);
    }

    async fn stop(&mut self) {
        self.grpc_endpoint = None;
        self.ui_url = None;
        self.events.lock().unwrap().push(Event::TemporalStop);
    }

    fn grpc_endpoint(&self) -> Option<&str> {
        self.grpc_endpoint.as_deref()
    }

    fn ui_url(&self) -> Option<&str> {
        self.ui_url.as_deref()
    }
}

struct OkReadinessCheck;

#[async_trait]
impl ReadinessCheck for OkReadinessCheck {
    async fn is_ready(
        &self,
        _identifier: &str,
        _grpc_endpoint: &str,
        _timeout_ms: u64,
    ) -> Result<(), String> {
        Ok(())
    }
}

struct PanickingTemporalReadinessCheck;

#[async_trait]
impl ReadinessCheck for PanickingTemporalReadinessCheck {
    async fn is_ready(
        &self,
        _identifier: &str,
        _grpc_endpoint: &str,
        _timeout_ms: u64,
    ) -> Result<(), String> {
        panic!("readiness probe failed");
    }
}

fn temporal_stop_count(events: &[Event]) -> usize {
    events
        .iter()
        .filter(|event| matches!(event, Event::TemporalStop))
        .count()
}

fn build_temporal(events: Arc<Mutex<Vec<Event>>>) -> TemporalDependency {
    TemporalDependency::builder("temporal-drop")
        .with_impl(FakeTemporalImpl {
            grpc_endpoint: None,
            ui_url: None,
            events,
        })
        .with_readiness_check(OkReadinessCheck)
        .build()
}

#[test]
fn drop_unstarted_dep_skips_impl_stop() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let dep = build_temporal(events.clone());
    drop(dep);
    assert_eq!(temporal_stop_count(&events.lock().unwrap()), 0);
}

#[tokio::test]
async fn stop_then_drop_single_impl_stop() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let mut dep = build_temporal(events.clone());
    dep.start().await;
    dep.stop().await;
    drop(dep);
    assert_eq!(temporal_stop_count(&events.lock().unwrap()), 1);
}

#[tokio::test]
async fn drop_running_dep_invokes_full_stop() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let mut dep = build_temporal(events.clone());
    dep.start().await;
    drop(dep);
    assert_eq!(temporal_stop_count(&events.lock().unwrap()), 1);
}

#[tokio::test]
async fn start_panic_then_drop_impl_stop() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let mut dep = TemporalDependency::builder("temporal-drop")
        .with_impl(FakeTemporalImpl {
            grpc_endpoint: None,
            ui_url: None,
            events: events.clone(),
        })
        .with_readiness_check(PanickingTemporalReadinessCheck)
        .build();

    let start_outcome = std::panic::AssertUnwindSafe(async {
        dep.start().await;
    })
    .catch_unwind()
    .await;
    assert!(start_outcome.is_err());
    assert_eq!(events.lock().unwrap().as_slice(), &[Event::TemporalStart]);

    drop(dep);
    assert_eq!(temporal_stop_count(&events.lock().unwrap()), 1);
}
