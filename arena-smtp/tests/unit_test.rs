use arena::dependency::RunnableDependency;
use arena::healthcheck::ReadinessCheck;
use arena_smtp::{SmtpDependency, SmtpImpl};
use async_trait::async_trait;
use futures::FutureExt;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq, Eq)]
enum Event {
    SmtpStart,
    SmtpStop,
    ReadinessCheck,
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

struct FakeReadinessCheck {
    events: Arc<Mutex<Vec<Event>>>,
    last_identifier: Arc<Mutex<Option<String>>>,
    last_smtp_address: Arc<Mutex<Option<String>>>,
    last_timeout_ms: Arc<Mutex<Option<u64>>>,
}

#[async_trait]
impl ReadinessCheck for FakeReadinessCheck {
    async fn is_ready(
        &self,
        identifier: &str,
        smtp_address: &str,
        timeout_ms: u64,
    ) -> Result<(), String> {
        self.events.lock().unwrap().push(Event::ReadinessCheck);
        *self.last_identifier.lock().unwrap() = Some(identifier.to_string());
        *self.last_smtp_address.lock().unwrap() = Some(smtp_address.to_string());
        *self.last_timeout_ms.lock().unwrap() = Some(timeout_ms);
        Ok(())
    }
}

struct FailingReadinessCheck;

#[async_trait]
impl ReadinessCheck for FailingReadinessCheck {
    async fn is_ready(
        &self,
        _identifier: &str,
        _smtp_address: &str,
        _timeout_ms: u64,
    ) -> Result<(), String> {
        Err("readiness failed".to_string())
    }
}

#[tokio::test]
async fn start_stop_happy_path_records_events() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let last_identifier = Arc::new(Mutex::new(None::<String>));
    let last_smtp_address = Arc::new(Mutex::new(None::<String>));
    let last_timeout_ms = Arc::new(Mutex::new(None::<u64>));

    let mut smtp = SmtpDependency::builder("smtp")
        .with_impl(FakeSmtpImpl {
            smtp_address: None,
            http_api_url: None,
            events: events.clone(),
        })
        .with_readiness_check(FakeReadinessCheck {
            events: events.clone(),
            last_identifier: last_identifier.clone(),
            last_smtp_address: last_smtp_address.clone(),
            last_timeout_ms: last_timeout_ms.clone(),
        })
        .build();

    let outcome = std::panic::AssertUnwindSafe(async {
        smtp.start().await;
        smtp.stop().await;
    })
    .catch_unwind()
    .await;

    assert!(outcome.is_ok(), "expected start/stop not to panic");

    let got = events.lock().unwrap().clone();
    assert_eq!(
        got,
        vec![Event::SmtpStart, Event::ReadinessCheck, Event::SmtpStop]
    );

    assert_eq!(
        last_identifier.lock().unwrap().as_deref(),
        Some(smtp.identifier.as_str())
    );
    assert_eq!(
        last_smtp_address.lock().unwrap().as_deref(),
        Some("127.0.0.1:1025")
    );
    let timeout_ms = last_timeout_ms
        .lock()
        .unwrap()
        .expect("readiness check should have been called with a timeout");
    assert!(
        timeout_ms <= 30_000,
        "expected timeout_ms <= 30_000, got {timeout_ms}"
    );
}

#[tokio::test]
async fn start_readiness_err_panics_after_impl_start() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let mut dep = SmtpDependency::builder("smtp")
        .with_impl(FakeSmtpImpl {
            smtp_address: None,
            http_api_url: None,
            events: events.clone(),
        })
        .with_readiness_check(FailingReadinessCheck)
        .build();

    let outcome = std::panic::AssertUnwindSafe(async {
        dep.start().await;
    })
    .catch_unwind()
    .await;

    assert!(outcome.is_err());
    assert_eq!(events.lock().unwrap().as_slice(), &[Event::SmtpStart]);
}
