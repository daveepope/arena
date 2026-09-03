use arena::dependency::RunnableDependency;
use arena_http::{HttpDependency, HttpImpl};
use async_trait::async_trait;
use futures::FutureExt;

struct FakeHttpImpl {
    base_url: Option<String>,
    started_base_url: String,
}

fn setup_fake_impl(started_base_url: &str) -> FakeHttpImpl {
    FakeHttpImpl {
        base_url: None,
        started_base_url: started_base_url.to_string(),
    }
}

#[async_trait]
impl HttpImpl for FakeHttpImpl {
    async fn start(&mut self, _port: u16, _image_name: &str, _image_tag: &str, _container_name: &str) {
        self.base_url = Some(self.started_base_url.clone());
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

#[tokio::test]
#[should_panic(expected = "reqwest client for HTTP admin API")]
async fn reset_journal_malformed_pem_panics_before_network_call() {
    let mut dep = HttpDependency::builder("http-bad-pem")
        .with_impl(setup_fake_impl("https://127.0.0.1:8443"))
        .with_port(0)
        .with_trusted_certificate_pem(
            "-----BEGIN CERTIFICATE-----\nbm90LWEtcmVhbC1jZXJ0\n-----END CERTIFICATE-----",
        )
        .with_readiness_check(OkReadinessCheck)
        .build();
    dep.start().await;

    dep.reset_journal().await;
}

#[tokio::test]
#[should_panic(expected = "reqwest client for HTTP admin API")]
async fn soft_reset_malformed_pem_panics_before_network_call() {
    let mut dep = HttpDependency::builder("http-bad-pem-soft")
        .with_impl(setup_fake_impl("https://127.0.0.1:8443"))
        .with_port(0)
        .with_trusted_certificate_pem(
            "-----BEGIN CERTIFICATE-----\nbm90LWEtcmVhbC1jZXJ0\n-----END CERTIFICATE-----",
        )
        .with_readiness_check(OkReadinessCheck)
        .build();
    dep.start().await;

    dep.soft_reset().await;
}

#[tokio::test]
async fn reset_journal_blank_pem_treated_as_absent_fails_over_network() {
    let mut dep = HttpDependency::builder("http-blank-pem")
        .with_impl(setup_fake_impl("https://127.0.0.1:8443"))
        .with_port(0)
        .with_trusted_certificate_pem("   ")
        .with_readiness_check(OkReadinessCheck)
        .build();
    dep.start().await;

    let outcome = std::panic::AssertUnwindSafe(dep.reset_journal())
        .catch_unwind()
        .await;

    assert!(outcome.is_err());
}

#[tokio::test]
#[should_panic(expected = "with_trusted_certificate_pem")]
async fn reset_journal_remote_tls_host_without_pem_panics() {
    let mut dep = HttpDependency::builder("http-remote-tls")
        .with_impl(setup_fake_impl("https://198.51.100.7:8443"))
        .with_port(0)
        .with_readiness_check(OkReadinessCheck)
        .build();
    dep.start().await;

    dep.reset_journal().await;
}

#[tokio::test]
async fn reset_journal_loopback_tls_host_without_pem_reaches_network() {
    let mut dep = HttpDependency::builder("http-loopback-tls")
        .with_impl(setup_fake_impl("https://localhost:8443"))
        .with_port(0)
        .with_readiness_check(OkReadinessCheck)
        .build();
    dep.start().await;

    let outcome = std::panic::AssertUnwindSafe(dep.reset_journal())
        .catch_unwind()
        .await;

    let message = outcome.expect_err("reset_journal should fail");
    let text = message
        .downcast_ref::<String>()
        .cloned()
        .unwrap_or_default();
    assert!(text.contains("reset_journal failed"), "{text}");
}
