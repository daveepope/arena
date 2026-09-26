use arena::lifecycle::{Fault, RunnableState};
use arena::dependency::{Dependency, RunnableDependency};
use arena::healthcheck::ReadinessCheck;
use arena_smtp::{SmtpDependency, SmtpImpl, SmtpTlsConfig, SmtpTlsMode};
use async_trait::async_trait;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Default)]
struct StartArgs {
    smtp_port: u16,
    ui_port: u16,
    image_name: String,
    image_tag: String,
    container_name: String,
    tls: Option<SmtpTlsConfig>,
}

struct RecordingSmtpImpl {
    recorded: Arc<Mutex<Option<StartArgs>>>,
    smtp_address: Option<String>,
    http_api_url: Option<String>,
}

#[async_trait]
impl SmtpImpl for RecordingSmtpImpl {
    async fn start(
        &mut self,
        smtp_port: u16,
        ui_port: u16,
        image_name: &str,
        image_tag: &str,
        container_name: &str,
        tls: Option<&SmtpTlsConfig>,
    ) -> Result<(), String> {
        *self.recorded.lock().unwrap() = Some(StartArgs {
            smtp_port,
            ui_port,
            image_name: image_name.to_string(),
            image_tag: image_tag.to_string(),
            container_name: container_name.to_string(),
            tls: tls.cloned(),
        });
        self.smtp_address = Some("127.0.0.1:1025".to_string());
        self.http_api_url = Some("http://127.0.0.1:8025".to_string());
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), String> {
        self.smtp_address = None;
        self.http_api_url = None;
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

#[derive(Debug, Clone, PartialEq, Eq)]
enum ChildEvent {
    Start,
    Stop,
}

struct RecordingChildDependency {
    events: Arc<Mutex<Vec<ChildEvent>>>,
}

#[async_trait]
impl RunnableDependency for RecordingChildDependency {
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
async fn with_port_and_ui_port_propagate_to_impl_start() {
    let recorded = Arc::new(Mutex::new(None::<StartArgs>));
    let mut dep = SmtpDependency::builder("builder-ports")
        .with_port(11111)
        .with_ui_port(22222)
        .with_impl(RecordingSmtpImpl {
            recorded: recorded.clone(),
            smtp_address: None,
            http_api_url: None,
        })
        .with_readiness_check(OkReadinessCheck)
        .build();

    dep.start().await.expect("start should succeed");
    dep.stop().await.expect("stop should succeed");

    let args = recorded
        .lock()
        .unwrap()
        .clone()
        .expect("start should have recorded args");
    assert_eq!(args.smtp_port, 11111);
    assert_eq!(args.ui_port, 22222);
}

#[tokio::test]
async fn with_image_name_and_tag_propagate_to_impl_start() {
    let recorded = Arc::new(Mutex::new(None::<StartArgs>));
    let mut dep = SmtpDependency::builder("builder-image")
        .with_image_name("example.com/custom-smtp")
        .with_image_tag("9.9.9")
        .with_impl(RecordingSmtpImpl {
            recorded: recorded.clone(),
            smtp_address: None,
            http_api_url: None,
        })
        .with_readiness_check(OkReadinessCheck)
        .build();

    dep.start().await.expect("start should succeed");
    dep.stop().await.expect("stop should succeed");

    let args = recorded
        .lock()
        .unwrap()
        .clone()
        .expect("start should have recorded args");
    assert_eq!(args.image_name, "example.com/custom-smtp");
    assert_eq!(args.image_tag, "9.9.9");
}

#[tokio::test]
async fn with_image_alias_sets_same_tag_as_with_image_tag() {
    let recorded = Arc::new(Mutex::new(None::<StartArgs>));
    let mut dep = SmtpDependency::builder("builder-image-alias")
        .with_image("1.30.5")
        .with_impl(RecordingSmtpImpl {
            recorded: recorded.clone(),
            smtp_address: None,
            http_api_url: None,
        })
        .with_readiness_check(OkReadinessCheck)
        .build();

    dep.start().await.expect("start should succeed");
    dep.stop().await.expect("stop should succeed");

    let args = recorded
        .lock()
        .unwrap()
        .clone()
        .expect("start should have recorded args");
    assert_eq!(args.image_tag, "1.30.5");
}

#[tokio::test]
async fn with_container_tag_alias_sets_same_tag_as_with_image_tag() {
    let recorded = Arc::new(Mutex::new(None::<StartArgs>));
    let mut dep = SmtpDependency::builder("builder-container-tag-alias")
        .with_container_tag("2.0.0")
        .with_impl(RecordingSmtpImpl {
            recorded: recorded.clone(),
            smtp_address: None,
            http_api_url: None,
        })
        .with_readiness_check(OkReadinessCheck)
        .build();

    dep.start().await.expect("start should succeed");
    dep.stop().await.expect("stop should succeed");

    let args = recorded
        .lock()
        .unwrap()
        .clone()
        .expect("start should have recorded args");
    assert_eq!(args.image_tag, "2.0.0");
}

#[tokio::test]
async fn with_container_name_propagates_to_impl_start() {
    let recorded = Arc::new(Mutex::new(None::<StartArgs>));
    let mut dep = SmtpDependency::builder("builder-container-name")
        .with_container_name("my-smtp-container")
        .with_impl(RecordingSmtpImpl {
            recorded: recorded.clone(),
            smtp_address: None,
            http_api_url: None,
        })
        .with_readiness_check(OkReadinessCheck)
        .build();

    dep.start().await.expect("start should succeed");
    dep.stop().await.expect("stop should succeed");

    let args = recorded
        .lock()
        .unwrap()
        .clone()
        .expect("start should have recorded args");
    assert_eq!(args.container_name, "my-smtp-container");
}

#[tokio::test]
async fn with_child_dependencies_starts_and_stops_children() {
    let events = Arc::new(Mutex::new(Vec::<ChildEvent>::new()));
    let mut dep = SmtpDependency::builder("builder-children")
        .with_child_dependencies(vec![Box::new(RecordingChildDependency {
            events: events.clone(),
        })])
        .with_impl(RecordingSmtpImpl {
            recorded: Arc::new(Mutex::new(None)),
            smtp_address: None,
            http_api_url: None,
        })
        .with_readiness_check(OkReadinessCheck)
        .build();

    dep.start().await.expect("start should succeed");
    dep.stop().await.expect("stop should succeed");

    assert_eq!(
        events.lock().unwrap().as_slice(),
        &[ChildEvent::Start, ChildEvent::Stop]
    );
}

#[tokio::test]
async fn with_starttls_passes_generated_tls_to_impl_start() {
    let recorded = Arc::new(Mutex::new(None::<StartArgs>));
    let mut dep = SmtpDependency::builder("builder-starttls")
        .with_starttls()
        .with_impl(RecordingSmtpImpl {
            recorded: recorded.clone(),
            smtp_address: None,
            http_api_url: None,
        })
        .with_readiness_check(OkReadinessCheck)
        .build();

    dep.start().await.expect("start should succeed");
    dep.stop().await.expect("stop should succeed");

    let tls = recorded
        .lock()
        .unwrap()
        .clone()
        .expect("start should have recorded args")
        .tls
        .expect("starttls should generate tls files");
    assert_eq!(tls.mode, SmtpTlsMode::StartTls);
    assert!(!tls.certificate_pem.is_empty());
    assert!(!tls.private_key_pem.is_empty());
    assert_ne!(tls.private_key_pem, tls.certificate_pem);
}

#[tokio::test]
async fn without_starttls_passes_no_tls_to_impl_start() {
    let recorded = Arc::new(Mutex::new(None::<StartArgs>));
    let mut dep = SmtpDependency::builder("builder-no-starttls")
        .with_impl(RecordingSmtpImpl {
            recorded: recorded.clone(),
            smtp_address: None,
            http_api_url: None,
        })
        .with_readiness_check(OkReadinessCheck)
        .build();

    dep.start().await.expect("start should succeed");
    dep.stop().await.expect("stop should succeed");

    let args = recorded
        .lock()
        .unwrap()
        .clone()
        .expect("start should have recorded args");
    assert!(args.tls.is_none());
}

#[tokio::test]
async fn with_implicit_tls_passes_generated_tls_to_impl_start() {
    let recorded = Arc::new(Mutex::new(None::<StartArgs>));
    let mut dep = SmtpDependency::builder("builder-implicit-tls")
        .with_implicit_tls()
        .with_impl(RecordingSmtpImpl {
            recorded: recorded.clone(),
            smtp_address: None,
            http_api_url: None,
        })
        .with_readiness_check(OkReadinessCheck)
        .build();

    dep.start().await.expect("start should succeed");
    dep.stop().await.expect("stop should succeed");

    let tls = recorded
        .lock()
        .unwrap()
        .clone()
        .expect("start should have recorded args")
        .tls
        .expect("implicit tls should generate tls files");
    assert_eq!(tls.mode, SmtpTlsMode::Implicit);
    assert!(!tls.certificate_pem.is_empty());
    assert!(!tls.private_key_pem.is_empty());
    assert_ne!(tls.private_key_pem, tls.certificate_pem);
}


#[derive(Clone, Default)]
struct ExpiryRecordingImpl {
    expiry: std::sync::Arc<std::sync::Mutex<Option<Option<std::time::Duration>>>>,
}

#[async_trait]
impl SmtpImpl for ExpiryRecordingImpl {
    fn set_expiry(&mut self, expiry: Option<std::time::Duration>) {
        *self.expiry.lock().unwrap() = Some(expiry);
    }
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
        None
    }
    fn http_api_url(&self) -> Option<&str> {
        None
    }
}

#[test]
fn build_no_expiry_override_uses_default_expiry() {
    let recorder = ExpiryRecordingImpl::default();
    let _dep = SmtpDependency::builder("orders").with_impl(recorder.clone()).build();

    assert_eq!(
        *recorder.expiry.lock().unwrap(),
        Some(Some(arena_container::expiry::DEFAULT_EXPIRY))
    );
}

#[test]
fn build_with_expiry_uses_given_expiry() {
    let recorder = ExpiryRecordingImpl::default();
    let _dep = SmtpDependency::builder("orders")
        .with_impl(recorder.clone())
        .with_expiry(std::time::Duration::from_secs(30))
        .build();

    assert_eq!(
        *recorder.expiry.lock().unwrap(),
        Some(Some(std::time::Duration::from_secs(30)))
    );
}

#[test]
fn build_without_expiry_disables_expiry() {
    let recorder = ExpiryRecordingImpl::default();
    let _dep = SmtpDependency::builder("orders").with_impl(recorder.clone()).without_expiry().build();

    assert_eq!(*recorder.expiry.lock().unwrap(), Some(None));
}
