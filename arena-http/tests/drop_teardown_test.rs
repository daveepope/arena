use arena::dependency::RunnableDependency;
use arena::healthcheck::ReadinessCheck;
use arena_http::{HttpDependency, HttpImpl};
use async_trait::async_trait;
use futures::FutureExt;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq, Eq)]
enum Event {
    HttpStart,
    HttpStop,
}

struct FakeHttpImpl {
    base_url: Option<String>,
    events: Arc<Mutex<Vec<Event>>>,
}

#[async_trait]
impl HttpImpl for FakeHttpImpl {
    async fn start(
        &mut self,
        _port: u16,
        _image_name: &str,
        _image_tag: &str,
        _container_name: &str,
    ) {
        self.base_url = Some("http://127.0.0.1:8080".to_string());
        self.events.lock().unwrap().push(Event::HttpStart);
    }

    async fn stop(&mut self) {
        self.base_url = None;
        self.events.lock().unwrap().push(Event::HttpStop);
    }

    fn base_url(&self) -> Option<&str> {
        self.base_url.as_deref()
    }

    fn admin_url(&self) -> Option<String> {
        self.base_url.as_deref().map(|url| format!("{url}/__admin"))
    }
}

struct OkReadinessCheck;

#[async_trait]
impl ReadinessCheck for OkReadinessCheck {
    async fn is_ready(
        &self,
        _identifier: &str,
        _admin_url: &str,
        _timeout_ms: u64,
    ) -> Result<(), String> {
        Ok(())
    }
}

struct PanickingHttpReadinessCheck;

#[async_trait]
impl ReadinessCheck for PanickingHttpReadinessCheck {
    async fn is_ready(
        &self,
        _identifier: &str,
        _admin_url: &str,
        _timeout_ms: u64,
    ) -> Result<(), String> {
        panic!("readiness probe failed");
    }
}

fn http_stop_count(events: &[Event]) -> usize {
    events
        .iter()
        .filter(|event| matches!(event, Event::HttpStop))
        .count()
}

fn build_http(events: Arc<Mutex<Vec<Event>>>) -> HttpDependency {
    HttpDependency::builder("http-drop")
        .with_impl(FakeHttpImpl {
            base_url: None,
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
    let dep = build_http(events.clone());
    drop(dep);
    assert_eq!(http_stop_count(&events.lock().unwrap()), 0);
}

#[tokio::test]
async fn stop_then_drop_single_impl_stop() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let mut dep = build_http(events.clone());
    dep.start().await;
    dep.stop().await;
    drop(dep);
    assert_eq!(http_stop_count(&events.lock().unwrap()), 1);
}

#[tokio::test]
async fn drop_running_dep_invokes_full_stop() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let mut dep = build_http(events.clone());
    dep.start().await;
    drop(dep);
    assert_eq!(http_stop_count(&events.lock().unwrap()), 1);
}

#[tokio::test]
async fn start_panic_then_drop_impl_stop() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let mut dep = HttpDependency::builder("http-drop")
        .with_impl(FakeHttpImpl {
            base_url: None,
            events: events.clone(),
        })
        .with_port(0)
        .with_image_tag("x")
        .with_readiness_check(PanickingHttpReadinessCheck)
        .build();

    let start_outcome = std::panic::AssertUnwindSafe(async {
        dep.start().await;
    })
    .catch_unwind()
    .await;
    assert!(start_outcome.is_err());
    assert_eq!(events.lock().unwrap().as_slice(), &[Event::HttpStart]);

    drop(dep);
    assert_eq!(http_stop_count(&events.lock().unwrap()), 1);
}
