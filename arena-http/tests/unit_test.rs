use arena::dependency::{Dependency, RunnableDependency};
use arena::healthcheck::ReadinessCheck;
use arena_http::{HttpDependency, HttpImpl};
use async_trait::async_trait;
use futures::FutureExt;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq, Eq)]
enum Event {
    HttpStart,
    HttpStop,
    ReadinessCheck,
}

struct FakeHttpImpl {
    base_url: Option<String>,
    admin_url: Option<String>,
    https_base_url: Option<String>,
    events: Arc<Mutex<Vec<Event>>>,
}

#[async_trait]
impl HttpImpl for FakeHttpImpl {
    async fn start(&mut self, _port: u16, _image_name: &str, _image_tag: &str, _container_name: &str) {
        self.base_url = Some("http://127.0.0.1:8080".to_string());
        self.admin_url = Some("http://127.0.0.1:8081".to_string());
        self.events.lock().unwrap().push(Event::HttpStart);
    }

    async fn stop(&mut self) {
        self.events.lock().unwrap().push(Event::HttpStop);
    }

    fn base_url(&self) -> Option<&str> {
        self.base_url.as_deref()
    }

    fn admin_url(&self) -> Option<String> {
        self.admin_url.clone()
    }

    fn https_base_url(&self) -> Option<&str> {
        self.https_base_url.as_deref()
    }
}

struct FakeReadinessCheck {
    events: Arc<Mutex<Vec<Event>>>,
}

#[async_trait]
impl ReadinessCheck for FakeReadinessCheck {
    async fn is_ready(&self, _identifier: &str, _admin_url: &str, _timeout_ms: u64) -> Result<(), String> {
        self.events.lock().unwrap().push(Event::ReadinessCheck);
        Ok(())
    }
}

struct FailingReadinessCheck;

#[async_trait]
impl ReadinessCheck for FailingReadinessCheck {
    async fn is_ready(&self, _identifier: &str, _admin_url: &str, _timeout_ms: u64) -> Result<(), String> {
        Err("readiness failed".to_string())
    }
}

struct FakeChildDependency {
    events: Arc<Mutex<Vec<&'static str>>>,
}

#[async_trait]
impl RunnableDependency for FakeChildDependency {
    fn identifier(&self) -> &str {
        "http-child"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    async fn start(&mut self) {
        self.events.lock().unwrap().push("start");
    }

    async fn stop(&mut self) {
        self.events.lock().unwrap().push("stop");
    }

    fn add_child(&mut self, _dep: Box<dyn RunnableDependency>) {}
    fn children(&self) -> &[Dependency] {
        &[]
    }
    fn children_mut(&mut self) -> &mut [Dependency] {
        &mut []
    }

    async fn soft_reset(&self) {}

    async fn hard_reset(&mut self) {}
}

#[tokio::test]
async fn start_stop_happy_path_records_events() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let child_events = Arc::new(Mutex::new(Vec::<&'static str>::new()));

    let mut dep = HttpDependency::builder("http")
        .with_impl(FakeHttpImpl {
            base_url: None,
            admin_url: None,
            https_base_url: None,
            events: events.clone(),
        })
        .with_port(0)
        .with_child_dependencies(vec![Box::new(FakeChildDependency {
            events: child_events.clone(),
        })])
        .with_readiness_check(FakeReadinessCheck {
            events: events.clone(),
        })
        .build();

    dep.start().await;
    assert_eq!(dep.base_url(), Some("http://127.0.0.1:8080"));
    dep.stop().await;

    assert_eq!(
        events.lock().unwrap().as_slice(),
        &[Event::HttpStart, Event::ReadinessCheck, Event::HttpStop]
    );
    assert_eq!(child_events.lock().unwrap().as_slice(), &["start", "stop"]);
}

#[tokio::test]
async fn start_readiness_err_panics_after_impl_start() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let mut dep = HttpDependency::builder("http")
        .with_impl(FakeHttpImpl {
            base_url: None,
            admin_url: None,
            https_base_url: None,
            events: events.clone(),
        })
        .with_port(0)
        .with_readiness_check(FailingReadinessCheck)
        .build();

    let outcome = std::panic::AssertUnwindSafe(async {
        dep.start().await;
    })
    .catch_unwind()
    .await;

    assert!(outcome.is_err());
    assert_eq!(events.lock().unwrap().as_slice(), &[Event::HttpStart]);
}

#[tokio::test]
async fn identifier_as_any_and_children_reflect_dependency_state() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let mut dep = HttpDependency::builder("http-accessors")
        .with_impl(FakeHttpImpl {
            base_url: None,
            admin_url: None,
            https_base_url: Some("https://127.0.0.1:8443".to_string()),
            events: events.clone(),
        })
        .with_port(0)
        .with_trusted_certificate_pem("test-pem")
        .with_readiness_check(FakeReadinessCheck {
            events: events.clone(),
        })
        .build();

    assert!(dep.identifier().contains("http-accessors"));
    assert!(dep.as_any().downcast_ref::<HttpDependency>().is_some());
    assert!(dep.as_any_mut().downcast_mut::<HttpDependency>().is_some());
    assert!(dep.children().is_empty());
    assert_eq!(dep.trusted_certificate_pem(), Some("test-pem"));

    dep.add_child(Box::new(FakeChildDependency {
        events: Arc::new(Mutex::new(Vec::new())),
    }));

    assert_eq!(dep.children().len(), 1);
    assert_eq!(dep.children_mut().len(), 1);

    dep.start().await;
    assert_eq!(dep.https_base_url(), Some("https://127.0.0.1:8443"));
    let _playbook = dep.playbook();
    dep.stop().await;
}

struct FlakyHttpImpl {
    calls: AtomicUsize,
    ready_after: usize,
    admin_url: String,
}

#[async_trait]
impl HttpImpl for FlakyHttpImpl {
    async fn start(&mut self, _port: u16, _image_name: &str, _image_tag: &str, _container_name: &str) {}

    async fn stop(&mut self) {}

    fn base_url(&self) -> Option<&str> {
        None
    }

    fn admin_url(&self) -> Option<String> {
        let seen = self.calls.fetch_add(1, Ordering::SeqCst);
        if seen < self.ready_after {
            None
        } else {
            Some(self.admin_url.clone())
        }
    }
}

struct ImmediateReadinessCheck;

#[async_trait]
impl ReadinessCheck for ImmediateReadinessCheck {
    async fn is_ready(&self, _identifier: &str, _admin_url: &str, _timeout_ms: u64) -> Result<(), String> {
        Ok(())
    }
}

#[tokio::test]
async fn wait_until_ready_retries_until_impl_reports_admin_url() {
    let mut dep = HttpDependency::builder("http-flaky")
        .with_impl(FlakyHttpImpl {
            calls: AtomicUsize::new(0),
            ready_after: 2,
            admin_url: "http://127.0.0.1:8081".to_string(),
        })
        .with_port(0)
        .with_readiness_check(ImmediateReadinessCheck)
        .build();

    dep.start().await;
    assert_eq!(dep.admin_url(), Some("http://127.0.0.1:8081".to_string()));
    dep.stop().await;
}

struct DefaultHttpsUrlImpl {
    base_url: Option<String>,
}

#[async_trait]
impl HttpImpl for DefaultHttpsUrlImpl {
    async fn start(&mut self, _port: u16, _image_name: &str, _image_tag: &str, _container_name: &str) {
        self.base_url = Some("http://127.0.0.1:8080".to_string());
    }

    async fn stop(&mut self) {
        self.base_url = None;
    }

    fn base_url(&self) -> Option<&str> {
        self.base_url.as_deref()
    }

    fn admin_url(&self) -> Option<String> {
        self.base_url.as_deref().map(|url| format!("{url}/__admin"))
    }
}

#[test]
fn https_base_url_default_trait_impl_returns_none() {
    let dep = HttpDependency::builder("http-default-https")
        .with_impl(DefaultHttpsUrlImpl { base_url: None })
        .with_port(0)
        .with_readiness_check(ImmediateReadinessCheck)
        .build();

    assert_eq!(dep.https_base_url(), None);
}

#[tokio::test]
async fn reset_journal_not_running_returns_without_panic() {
    let dep = HttpDependency::builder("http-reset-not-running")
        .with_impl(DefaultHttpsUrlImpl { base_url: None })
        .with_port(0)
        .with_readiness_check(ImmediateReadinessCheck)
        .build();

    dep.reset_journal().await;
}

#[tokio::test]
async fn soft_reset_not_running_returns_without_panic() {
    let dep = HttpDependency::builder("http-soft-reset-not-running")
        .with_impl(DefaultHttpsUrlImpl { base_url: None })
        .with_port(0)
        .with_readiness_check(ImmediateReadinessCheck)
        .build();

    dep.soft_reset().await;
}

#[tokio::test]
async fn hard_reset_not_running_returns_without_panic() {
    let mut dep = HttpDependency::builder("http-hard-reset-not-running")
        .with_impl(DefaultHttpsUrlImpl { base_url: None })
        .with_port(0)
        .with_readiness_check(ImmediateReadinessCheck)
        .build();

    dep.hard_reset().await;
}

#[tokio::test]
async fn hard_reset_running_restarts_impl_stays_ready() {
    let mut dep = HttpDependency::builder("http-hard-reset")
        .with_impl(DefaultHttpsUrlImpl { base_url: None })
        .with_port(0)
        .with_readiness_check(ImmediateReadinessCheck)
        .build();

    dep.start().await;
    assert_eq!(dep.base_url(), Some("http://127.0.0.1:8080"));

    dep.hard_reset().await;
    assert_eq!(dep.base_url(), Some("http://127.0.0.1:8080"));

    dep.stop().await;
}
