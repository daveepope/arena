use arena::dependency::RunnableDependency;
use arena::healthcheck::ReadinessCheck;
use arena::lifecycle::RunnableState;
use arena_smtp::{SmtpDependency, SmtpImpl, SmtpTlsConfig};
use async_trait::async_trait;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq, Eq)]
enum Event {
    SmtpStart,
    SmtpStop,
    SmtpForceStop,
    SmtpRelease,
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
        self.smtp_address = None;
        self.http_api_url = None;
        self.events.lock().unwrap().push(Event::SmtpForceStop);
        true
    }
    fn release(&mut self) {
        self.events.lock().unwrap().push(Event::SmtpRelease);
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

struct FailingSmtpReadinessCheck;

#[async_trait]
impl ReadinessCheck for FailingSmtpReadinessCheck {
    async fn is_ready(
        &self,
        _identifier: &str,
        _smtp_address: &str,
        _timeout_ms: u64,
    ) -> Result<(), String> {
        Err("readiness probe failed".to_string())
    }
}

fn count_of(events: &[Event], wanted: Event) -> usize {
    events.iter().filter(|event| **event == wanted).count()
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

fn build_smtp_with_failing_readiness(events: Arc<Mutex<Vec<Event>>>) -> SmtpDependency {
    SmtpDependency::builder("smtp-drop")
        .with_impl(FakeSmtpImpl {
            smtp_address: None,
            http_api_url: None,
            events,
        })
        .with_readiness_check(FailingSmtpReadinessCheck)
        .build()
}

#[test]
fn drop_unstarted_dependency_skips_teardown() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let dep = build_smtp(events.clone());

    drop(dep);

    assert!(events.lock().unwrap().is_empty());
}

#[tokio::test]
async fn start_healthy_dependency_returns_started() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let mut dep = build_smtp(events.clone());

    dep.start().await.expect("dependency should start");

    assert_eq!(dep.state(), RunnableState::Started);
    assert_eq!(count_of(&events.lock().unwrap(), Event::SmtpStart), 1);
}

#[tokio::test]
async fn stop_started_dependency_returns_stopped() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let mut dep = build_smtp(events.clone());

    dep.start().await.expect("dependency should start");
    dep.stop().await.expect("dependency should stop");

    assert_eq!(dep.state(), RunnableState::Stopped);
    assert_eq!(count_of(&events.lock().unwrap(), Event::SmtpStop), 1);
}

#[tokio::test]
async fn stop_then_drop_does_not_stop_twice() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let mut dep = build_smtp(events.clone());

    dep.start().await.expect("dependency should start");
    dep.stop().await.expect("dependency should stop");
    drop(dep);

    assert_eq!(count_of(&events.lock().unwrap(), Event::SmtpStop), 1);
    assert_eq!(count_of(&events.lock().unwrap(), Event::SmtpForceStop), 0);
}

#[tokio::test]
async fn drop_running_dependency_releases_container() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let mut dep = build_smtp(events.clone());

    dep.start().await.expect("dependency should start");
    drop(dep);

    assert_eq!(count_of(&events.lock().unwrap(), Event::SmtpRelease), 1);
}

#[tokio::test]
async fn start_readiness_failure_returns_fault_and_forces_stop() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let mut dep = build_smtp_with_failing_readiness(events.clone());

    let fault = dep.start().await.expect_err("dependency should fault");

    assert_eq!(fault.id, dep.identifier());
    assert!(fault.message.contains("readiness check failed"));
    assert_eq!(dep.state(), RunnableState::Stopped);
    assert_eq!(dep.faults().len(), 1);
    assert_eq!(count_of(&events.lock().unwrap(), Event::SmtpForceStop), 1);
}

#[tokio::test]
async fn start_readiness_failure_then_drop_does_not_force_stop_twice() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let mut dep = build_smtp_with_failing_readiness(events.clone());

    let _fault = dep.start().await.expect_err("dependency should fault");
    drop(dep);

    assert_eq!(count_of(&events.lock().unwrap(), Event::SmtpForceStop), 1);
}

#[tokio::test]
async fn force_stop_called_twice_is_indistinguishable_from_once() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let mut dep = build_smtp(events.clone());

    dep.start().await.expect("dependency should start");
    dep.force_stop().await;
    let after_first = dep.state();
    dep.force_stop().await;

    assert_eq!(after_first, RunnableState::Stopped);
    assert_eq!(dep.state(), RunnableState::Stopped);
    assert!(dep.faults().is_empty());
}
