use arena::lifecycle::{Fault, RunnableState};
use arena::dependency::{Dependency, RunnableDependency};
use arena::healthcheck::ReadinessCheck;
use arena_smtp::{SmtpDependency, SmtpImpl, SmtpTlsConfig};
use async_trait::async_trait;
use futures::FutureExt;
use std::sync::atomic::{AtomicU32, Ordering};
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
        _tls: Option<&SmtpTlsConfig>,
    ) -> Result<(), String> {
        self.smtp_address = Some("127.0.0.1:1025".to_string());
        self.http_api_url = Some("http://127.0.0.1:8025".to_string());
        self.events.lock().unwrap().push(Event::SmtpStart);
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), String> {
        self.smtp_address = None;
        self.http_api_url = None;
        self.events.lock().unwrap().push(Event::SmtpStop);
        Ok(())
    }
    async fn force_stop(&mut self) -> bool {
        true
    }
    fn release(&mut self) {}


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

    let http_api_url_while_running = Arc::new(Mutex::new(None::<String>));
    let http_api_url_while_running_write = http_api_url_while_running.clone();

    let outcome = std::panic::AssertUnwindSafe(async {
        smtp.start().await.expect("start should succeed");
        *http_api_url_while_running_write.lock().unwrap() =
            smtp.http_api_url().map(str::to_string);
        smtp.stop().await.expect("stop should succeed");
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
    assert_eq!(
        http_api_url_while_running.lock().unwrap().as_deref(),
        Some("http://127.0.0.1:8025")
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
        dep.start().await.expect("start should succeed");
    })
    .catch_unwind()
    .await;

    assert!(outcome.is_err());
    assert_eq!(events.lock().unwrap().as_slice(), &[Event::SmtpStart]);
}

struct AlwaysOkReadinessCheck;

#[async_trait]
impl ReadinessCheck for AlwaysOkReadinessCheck {
    async fn is_ready(
        &self,
        _identifier: &str,
        _smtp_address: &str,
        _timeout_ms: u64,
    ) -> Result<(), String> {
        Ok(())
    }
}

struct RetryingSmtpImpl {
    smtp_address: String,
    http_api_url: String,
    address_polls: AtomicU32,
    ready_after_polls: u32,
}

#[async_trait]
impl SmtpImpl for RetryingSmtpImpl {
    async fn start(
        &mut self,
        _smtp_port: u16,
        _ui_port: u16,
        _image_name: &str,
        _image_tag: &str,
        _container_name: &str,
        _tls: Option<&SmtpTlsConfig>,
    ) -> Result<(), String> {
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), String> {
        Ok(())
    }
    async fn force_stop(&mut self) -> bool {
        true
    }
    fn release(&mut self) {}


    fn smtp_address(&self) -> Option<&str> {
        let polls = self.address_polls.fetch_add(1, Ordering::SeqCst) + 1;
        if polls >= self.ready_after_polls {
            Some(&self.smtp_address)
        } else {
            None
        }
    }

    fn http_api_url(&self) -> Option<&str> {
        Some(&self.http_api_url)
    }
}

#[tokio::test]
async fn wait_until_ready_retries_until_impl_reports_address() {
    let mut dep = SmtpDependency::builder("smtp-retry")
        .with_impl(RetryingSmtpImpl {
            smtp_address: "127.0.0.1:1025".to_string(),
            http_api_url: "http://127.0.0.1:8025".to_string(),
            address_polls: AtomicU32::new(0),
            ready_after_polls: 3,
        })
        .with_readiness_check(AlwaysOkReadinessCheck)
        .build();

    dep.start().await.expect("start should succeed");
    dep.stop().await.expect("stop should succeed");
}

#[tokio::test]
async fn hard_reset_stops_and_restarts_impl() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let mut dep = SmtpDependency::builder("smtp-hard-reset")
        .with_impl(FakeSmtpImpl {
            smtp_address: None,
            http_api_url: None,
            events: events.clone(),
        })
        .with_readiness_check(AlwaysOkReadinessCheck)
        .build();

    dep.start().await.expect("start should succeed");
    dep.hard_reset().await.expect("hard reset should succeed");

    assert_eq!(
        events.lock().unwrap().as_slice(),
        &[Event::SmtpStart, Event::SmtpStop, Event::SmtpStart]
    );

    dep.stop().await.expect("stop should succeed");
}

#[tokio::test]
async fn soft_reset_before_start_is_noop() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let dep = SmtpDependency::builder("smtp-soft-reset-idle")
        .with_impl(FakeSmtpImpl {
            smtp_address: None,
            http_api_url: None,
            events: events.clone(),
        })
        .with_readiness_check(AlwaysOkReadinessCheck)
        .build();

    dep.soft_reset().await.expect("soft reset should succeed");

    assert!(events.lock().unwrap().is_empty());
}

#[tokio::test]
async fn soft_reset_while_running_does_not_panic() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let mut dep = SmtpDependency::builder("smtp-soft-reset-running")
        .with_impl(FakeSmtpImpl {
            smtp_address: None,
            http_api_url: None,
            events: events.clone(),
        })
        .with_readiness_check(AlwaysOkReadinessCheck)
        .build();

    dep.start().await.expect("start should succeed");
    dep.soft_reset().await.expect("soft reset should succeed");
    dep.stop().await.expect("stop should succeed");
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ChildEvent {
    Start,
    Stop,
}

struct RecordingChild {
    events: Arc<Mutex<Vec<ChildEvent>>>,
}

#[async_trait]
impl RunnableDependency for RecordingChild {
    fn identifier(&self) -> &str {
        "child"
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
        self.events.lock().unwrap().push(ChildEvent::Start);
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), Fault> {
        self.events.lock().unwrap().push(ChildEvent::Stop);
        Ok(())
    }

    async fn soft_reset(&self) -> Result<(), Fault> {
        Ok(())
    }

    async fn hard_reset(&mut self) -> Result<(), Fault> {
        Ok(())
    }

    fn add_child(&mut self, _dep: Box<dyn RunnableDependency>) {}
    fn children(&self) -> &[Dependency] {
        &[]
    }
    fn children_mut(&mut self) -> &mut [Dependency] {
        &mut []
    }
}

#[tokio::test]
async fn add_child_appends_and_lifecycle_includes_it() {
    let child_events = Arc::new(Mutex::new(Vec::<ChildEvent>::new()));
    let mut dep = SmtpDependency::builder("smtp-add-child")
        .with_impl(FakeSmtpImpl {
            smtp_address: None,
            http_api_url: None,
            events: Arc::new(Mutex::new(Vec::new())),
        })
        .with_readiness_check(AlwaysOkReadinessCheck)
        .build();

    assert!(dep.children().is_empty());

    dep.add_child(Box::new(RecordingChild {
        events: child_events.clone(),
    }));

    assert_eq!(dep.children().len(), 1);
    assert_eq!(dep.children_mut().len(), 1);

    dep.start().await.expect("start should succeed");
    dep.stop().await.expect("stop should succeed");

    assert_eq!(
        child_events.lock().unwrap().as_slice(),
        &[ChildEvent::Start, ChildEvent::Stop]
    );
}

#[tokio::test]
async fn identifier_and_any_casts_expose_dependency() {
    let mut dep = SmtpDependency::builder("smtp-any")
        .with_impl(FakeSmtpImpl {
            smtp_address: None,
            http_api_url: None,
            events: Arc::new(Mutex::new(Vec::new())),
        })
        .with_readiness_check(AlwaysOkReadinessCheck)
        .build();

    let expected_identifier = dep.identifier.clone();
    assert_eq!(RunnableDependency::identifier(&dep), expected_identifier);
    assert!(dep.as_any().downcast_ref::<SmtpDependency>().is_some());
    assert!(dep.as_any_mut().downcast_mut::<SmtpDependency>().is_some());
}

#[tokio::test]
async fn build_with_implicit_tls_generates_certificate_without_panicking() {
    let dep = SmtpDependency::builder("smtp-tls-build")
        .with_implicit_tls()
        .build();

    assert!(dep.faults().is_empty());
    assert_eq!(dep.state(), RunnableState::NotStarted);
}

#[tokio::test]
async fn build_without_tls_records_no_fault() {
    let dep = SmtpDependency::builder("smtp-no-tls").build();

    assert!(dep.faults().is_empty());
    assert_eq!(dep.state(), RunnableState::NotStarted);
}
