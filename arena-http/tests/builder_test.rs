use arena::dependency::RunnableDependency;
use arena_http::{HttpDependency, HttpImpl};
use async_trait::async_trait;
use std::time::{SystemTime, UNIX_EPOCH};

fn test_password() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time before unix epoch")
        .as_nanos();
    format!("pw-{nanos}")
}

struct FakeHttpImpl {
    base_url: Option<String>,
}

#[async_trait]
impl HttpImpl for FakeHttpImpl {
    async fn start(&mut self, _port: u16, _image_name: &str, _image_tag: &str, _container_name: &str) -> Result<(), String> {
        self.base_url = Some("http://127.0.0.1:8080".to_string());
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), String> {
        self.base_url = None;
        Ok(())
    }
    async fn force_stop(&mut self) -> bool {
        true
    }
    fn release(&mut self) {}


    fn base_url(&self) -> Option<&str> {
        self.base_url.as_deref()
    }

    fn admin_url(&self) -> Option<String> {
        self.base_url.as_deref().map(|url| format!("{url}/__admin"))
    }
}

struct OkReadinessCheck;

#[async_trait]
impl arena::healthcheck::ReadinessCheck for OkReadinessCheck {
    async fn is_ready(&self, _identifier: &str, _admin_url: &str, _timeout_ms: u64) -> Result<(), String> {
        Ok(())
    }
}

#[test]
fn build_with_impl_ignores_container_cli_config() {
    let dep = HttpDependency::builder("http-with-impl")
        .with_impl(FakeHttpImpl { base_url: None })
        .with_port(0)
        .with_image_name("custom-image")
        .with_image_tag("custom-tag")
        .with_image("legacy-image-tag")
        .with_container_name("custom-container")
        .with_container_tag("legacy-container-tag")
        .with_network("custom-network")
        .with_readiness_check(OkReadinessCheck)
        .build().expect("build http dependency");

    assert!(dep.identifier().contains("http-with-impl"));
}

#[tokio::test]
async fn https_listener_port_only_starts_reads_base_url() {
    let mut dep = HttpDependency::builder("http-https-listener")
        .with_impl(FakeHttpImpl { base_url: None })
        .with_port(0)
        .https()
        .listener_container_port(8443)
        .host_port(0)
        .done()
        .with_readiness_check(OkReadinessCheck)
        .build().expect("build http dependency");

    dep.start().await.expect("start should succeed");
    assert_eq!(dep.base_url(), Some("http://127.0.0.1:8080"));
    dep.stop().await.expect("stop should succeed");
}

#[tokio::test]
async fn https_full_keystore_config_starts_successfully() {
    let mut dep = HttpDependency::builder("http-https-keystore")
        .with_impl(FakeHttpImpl { base_url: None })
        .with_port(0)
        .https()
        .listener_container_port(8443)
        .keystore_path("/keystore.jks")
        .keystore_password(test_password())
        .key_password(test_password())
        .keystore_type("JKS")
        .http_listener_disabled(false)
        .done()
        .with_readiness_check(OkReadinessCheck)
        .build().expect("build http dependency");

    dep.start().await.expect("start should succeed");
    dep.stop().await.expect("stop should succeed");
}

#[test]
fn build_without_impl_builds_container_cli_config() {
    let dep = HttpDependency::builder("http-container-cli")
        .with_port(0)
        .with_network("cli-network")
        .https()
        .listener_container_port(8443)
        .host_port(9443)
        .keystore_path("/keystore.jks")
        .keystore_password(test_password())
        .key_password(test_password())
        .keystore_type("JKS")
        .http_listener_disabled(true)
        .done()
        .build().expect("build http dependency");

    assert!(dep.identifier().contains("http-container-cli"));
}

#[test]
fn build_without_impl_default_https_builds_container_impl() {
    let dep = HttpDependency::builder("http-container-default")
        .with_port(0)
        .build().expect("build http dependency");

    assert!(dep.identifier().contains("http-container-default"));
}

#[test]
fn build_https_disabled_without_listener_port_returns_fault() {
    let fault = HttpDependency::builder("http-https-bad-disable")
        .with_port(0)
        .https()
        .http_listener_disabled(true)
        .done()
        .build()
        .err()
        .expect("https misconfiguration must fault");
    assert!(fault
        .message
        .contains("http_listener_disabled(true) requires https().listener_container_port"));
}

#[test]
fn build_https_keystore_password_without_path_returns_fault() {
    let fault = HttpDependency::builder("http-https-bad-keystore-pw")
        .with_port(0)
        .https()
        .keystore_password(test_password())
        .done()
        .build()
        .err()
        .expect("https misconfiguration must fault");
    assert!(fault
        .message
        .contains("keystore password / key password / keystore type require"));
}

#[test]
fn build_https_keystore_path_without_listener_port_returns_fault() {
    let fault = HttpDependency::builder("http-https-bad-keystore-path")
        .with_port(0)
        .https()
        .keystore_path("/keystore.jks")
        .done()
        .build()
        .err()
        .expect("https misconfiguration must fault");
    assert!(fault
        .message
        .contains("https().keystore_path(...) requires https().listener_container_port"));
}


#[derive(Clone, Default)]
struct ExpiryRecordingImpl {
    expiry: std::sync::Arc<std::sync::Mutex<Option<Option<std::time::Duration>>>>,
}

#[async_trait]
impl HttpImpl for ExpiryRecordingImpl {
    fn set_expiry(&mut self, expiry: Option<std::time::Duration>) {
        *self.expiry.lock().unwrap() = Some(expiry);
    }
    async fn start(
        &mut self,
        _port: u16,
        _image_name: &str,
        _image_tag: &str,
        _container_name: &str,
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
    fn base_url(&self) -> Option<&str> {
        None
    }
    fn admin_url(&self) -> Option<String> {
        None
    }
}

#[test]
fn build_no_expiry_override_uses_default_expiry() {
    let recorder = ExpiryRecordingImpl::default();
    let _dep = HttpDependency::builder("orders").with_impl(recorder.clone()).build().expect("build http dependency");

    assert_eq!(
        *recorder.expiry.lock().unwrap(),
        Some(Some(arena_container::expiry::DEFAULT_EXPIRY))
    );
}

#[test]
fn build_with_expiry_uses_given_expiry() {
    let recorder = ExpiryRecordingImpl::default();
    let _dep = HttpDependency::builder("orders")
        .with_impl(recorder.clone())
        .with_expiry(std::time::Duration::from_secs(30))
        .build().expect("build http dependency");

    assert_eq!(
        *recorder.expiry.lock().unwrap(),
        Some(Some(std::time::Duration::from_secs(30)))
    );
}

#[test]
fn build_without_expiry_disables_expiry() {
    let recorder = ExpiryRecordingImpl::default();
    let _dep = HttpDependency::builder("orders").with_impl(recorder.clone()).without_expiry().build().expect("build http dependency");

    assert_eq!(*recorder.expiry.lock().unwrap(), Some(None));
}
