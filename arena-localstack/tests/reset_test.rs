use arena::dependency::RunnableDependency;
use arena::healthcheck::ReadinessCheck;
use arena_localstack::{LocalstackDependency, LocalstackImpl};
use async_trait::async_trait;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

struct CountingLocalstackImpl {
    endpoint: Option<String>,
    start_calls: Arc<AtomicUsize>,
    stop_calls: Arc<AtomicUsize>,
}

#[async_trait]
impl LocalstackImpl for CountingLocalstackImpl {
    async fn start(
        &mut self,
        _port: u16,
        _image_name: &str,
        _image_tag: &str,
        _container_name: &str,
        _services: &[String],
    ) -> Result<(), String> {
        self.endpoint = Some("http://127.0.0.1:4566".to_string());
        self.start_calls.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), String> {
        self.stop_calls.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
    async fn force_stop(&mut self) -> bool {
        true
    }
    fn release(&mut self) {}


    fn endpoint_url(&self) -> Option<&str> {
        self.endpoint.as_deref()
    }
}

struct OkReadinessCheck;

#[async_trait]
impl ReadinessCheck for OkReadinessCheck {
    async fn is_ready(
        &self,
        _identifier: &str,
        _endpoint: &str,
        _timeout_ms: u64,
    ) -> Result<(), String> {
        Ok(())
    }
}

fn build_dep(
    start_calls: Arc<AtomicUsize>,
    stop_calls: Arc<AtomicUsize>,
) -> LocalstackDependency {
    LocalstackDependency::builder("localstack-reset")
        .with_impl(CountingLocalstackImpl {
            endpoint: None,
            start_calls,
            stop_calls,
        })
        .with_port(0)
        .with_image_tag("x")
        .with_readiness_check(OkReadinessCheck)
        .build()
}

#[tokio::test]
async fn soft_reset_not_started_returns_without_calling_impl() {
    let start_calls = Arc::new(AtomicUsize::new(0));
    let stop_calls = Arc::new(AtomicUsize::new(0));
    let dep = build_dep(start_calls.clone(), stop_calls.clone());

    dep.soft_reset().await.expect("soft reset should succeed");

    assert_eq!(start_calls.load(Ordering::SeqCst), 0);
    assert_eq!(stop_calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn soft_reset_started_no_queues_completes() {
    let start_calls = Arc::new(AtomicUsize::new(0));
    let stop_calls = Arc::new(AtomicUsize::new(0));
    let mut dep = build_dep(start_calls.clone(), stop_calls.clone());

    dep.start().await.expect("start should succeed");
    dep.soft_reset().await.expect("soft reset should succeed");
    dep.stop().await.expect("stop should succeed");

    assert_eq!(start_calls.load(Ordering::SeqCst), 1);
    assert_eq!(stop_calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn hard_reset_not_started_returns_without_restart() {
    let start_calls = Arc::new(AtomicUsize::new(0));
    let stop_calls = Arc::new(AtomicUsize::new(0));
    let mut dep = build_dep(start_calls.clone(), stop_calls.clone());

    dep.hard_reset().await.expect("hard reset should succeed");

    assert_eq!(start_calls.load(Ordering::SeqCst), 0);
    assert_eq!(stop_calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn hard_reset_started_restarts_container() {
    let start_calls = Arc::new(AtomicUsize::new(0));
    let stop_calls = Arc::new(AtomicUsize::new(0));
    let mut dep = build_dep(start_calls.clone(), stop_calls.clone());

    dep.start().await.expect("start should succeed");
    dep.hard_reset().await.expect("hard reset should succeed");

    assert_eq!(start_calls.load(Ordering::SeqCst), 2);
    assert_eq!(stop_calls.load(Ordering::SeqCst), 1);
    assert_eq!(dep.endpoint_url(), Some("http://127.0.0.1:4566"));

    dep.stop().await.expect("stop should succeed");
    assert_eq!(stop_calls.load(Ordering::SeqCst), 2);
}
