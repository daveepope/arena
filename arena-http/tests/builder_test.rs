use arena::dependency::RunnableDependency;
use arena_http::{HttpDependency, HttpImpl};
use async_trait::async_trait;

struct FakeHttpImpl {
    base_url: Option<String>,
}

#[async_trait]
impl HttpImpl for FakeHttpImpl {
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
        .build();

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
        .build();

    dep.start().await;
    assert_eq!(dep.base_url(), Some("http://127.0.0.1:8080"));
    dep.stop().await;
}

#[tokio::test]
async fn https_full_keystore_config_starts_successfully() {
    let mut dep = HttpDependency::builder("http-https-keystore")
        .with_impl(FakeHttpImpl { base_url: None })
        .with_port(0)
        .https()
        .listener_container_port(8443)
        .keystore_path("/keystore.jks")
        .keystore_password("pw")
        .key_password("kpw")
        .keystore_type("JKS")
        .http_listener_disabled(false)
        .done()
        .with_readiness_check(OkReadinessCheck)
        .build();

    dep.start().await;
    dep.stop().await;
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
        .keystore_password("pw")
        .key_password("kpw")
        .keystore_type("JKS")
        .http_listener_disabled(true)
        .done()
        .build();

    assert!(dep.identifier().contains("http-container-cli"));
}

#[test]
fn build_without_impl_default_https_builds_container_impl() {
    let dep = HttpDependency::builder("http-container-default")
        .with_port(0)
        .build();

    assert!(dep.identifier().contains("http-container-default"));
}

#[test]
#[should_panic(expected = "http_listener_disabled(true) requires https().listener_container_port")]
fn https_disabled_without_listener_port_panics() {
    let _dep = HttpDependency::builder("http-https-bad-disable")
        .with_port(0)
        .https()
        .http_listener_disabled(true)
        .done()
        .build();
}

#[test]
#[should_panic(expected = "keystore password / key password / keystore type require")]
fn https_keystore_password_without_path_panics() {
    let _dep = HttpDependency::builder("http-https-bad-keystore-pw")
        .with_port(0)
        .https()
        .keystore_password("pw")
        .done()
        .build();
}

#[test]
#[should_panic(expected = "https().keystore_path(...) requires https().listener_container_port")]
fn https_keystore_path_without_listener_port_panics() {
    let _dep = HttpDependency::builder("http-https-bad-keystore-path")
        .with_port(0)
        .https()
        .keystore_path("/keystore.jks")
        .done()
        .build();
}
