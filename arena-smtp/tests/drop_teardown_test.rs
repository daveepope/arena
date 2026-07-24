use arena::dependency::RunnableDependency;
use arena::healthcheck::ReadinessCheck;
use arena_smtp::{SmtpDependency, SmtpImpl, SmtpTlsFiles};
use async_trait::async_trait;
use futures::FutureExt;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq, Eq)]
enum Event {
    SmtpStart,
    SmtpStop,
}

struct FakeSmtpImpl {
    smtp_address: Option<String>,
    http_api_url: Option<String>,
    events: Arc<Mutex<Vec<Event>>>,
}

#[async_trait]
impl SmtpImpl for FakeSmtpImpl {
    async fn start(
        &mut self,
        _smtp_port: u16,
        _ui_port: u16,
        _image_name: &str,
        _image_tag: &str,
        _container_name: &str,
        _tls: Option<&SmtpTlsFiles>,
    ) {
        self.smtp_address = Some("127.0.0.1:1025".to_string());
        self.http_api_url = Some("http://127.0.0.1:8025".to_string());
        self.events.lock().unwrap().push(Event::SmtpStart);
    }

    async fn stop(&mut self) {
        self.smtp_address = None;
        self.http_api_url = None;
        self.events.lock().unwrap().push(Event::SmtpStop);
    }

    fn smtp_address(&self) -> Option<&str> {
        self.smtp_address.as_deref()
    }

    fn http_api_url(&self) -> Option<&str> {
        self.http_api_url.as_deref()
    }
}

struct OkReadinessCheck;

#[async_trait]
impl ReadinessCheck for OkReadinessCheck {
    async fn is_ready(
        &self,
        _identifier: &str,
        _smtp_address: &str,
        _timeout_ms: u64,
    ) -> Result<(), String> {
        Ok(())
    }
}

struct PanickingSmtpReadinessCheck;

#[async_trait]
impl ReadinessCheck for PanickingSmtpReadinessCheck {
    async fn is_ready(
        &self,
        _identifier: &str,
        _smtp_address: &str,
        _timeout_ms: u64,
    ) -> Result<(), String> {
        panic!("readiness probe failed");
    }
}

fn smtp_stop_count(events: &[Event]) -> usize {
    events
        .iter()
        .filter(|event| matches!(event, Event::SmtpStop))
        .count()
}

fn build_smtp(events: Arc<Mutex<Vec<Event>>>) -> SmtpDependency {
    SmtpDependency::builder("smtp-drop")
        .with_impl(FakeSmtpImpl {
            smtp_address: None,
            http_api_url: None,
            events,
        })
        .with_readiness_check(OkReadinessCheck)
        .build()
}

#[test]
fn drop_unstarted_dep_skips_impl_stop() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let dep = build_smtp(events.clone());
    drop(dep);
    assert_eq!(smtp_stop_count(&events.lock().unwrap()), 0);
}

#[tokio::test]
async fn stop_then_drop_single_impl_stop() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let mut dep = build_smtp(events.clone());
    dep.start().await;
    dep.stop().await;
    drop(dep);
    assert_eq!(smtp_stop_count(&events.lock().unwrap()), 1);
}

#[tokio::test]
async fn drop_running_dep_invokes_full_stop() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let mut dep = build_smtp(events.clone());
    dep.start().await;
    drop(dep);
    assert_eq!(smtp_stop_count(&events.lock().unwrap()), 1);
}

#[tokio::test]
async fn start_panic_then_drop_impl_stop() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let mut dep = SmtpDependency::builder("smtp-drop")
        .with_impl(FakeSmtpImpl {
            smtp_address: None,
            http_api_url: None,
            events: events.clone(),
        })
        .with_readiness_check(PanickingSmtpReadinessCheck)
        .build();

    let start_outcome = std::panic::AssertUnwindSafe(async {
        dep.start().await;
    })
    .catch_unwind()
    .await;
    assert!(start_outcome.is_err());
    assert_eq!(events.lock().unwrap().as_slice(), &[Event::SmtpStart]);

    drop(dep);
    assert_eq!(smtp_stop_count(&events.lock().unwrap()), 1);
}
