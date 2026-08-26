use arena::dependency::{Dependency, RunnableDependency};
use arena::healthcheck::ReadinessCheck;
use arena_temporal::{TemporalDependency, TemporalImpl};
use async_trait::async_trait;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;
use tokio_stream::wrappers::TcpListenerStream;

#[derive(Debug, Clone, PartialEq, Eq)]
enum Event {
    TemporalStart,
    TemporalStop,
}

struct FakeTemporalImpl {
    grpc_endpoint: Option<String>,
    ui_url: Option<String>,
    events: Arc<Mutex<Vec<Event>>>,
}

#[async_trait]
impl TemporalImpl for FakeTemporalImpl {
    async fn start(
        &mut self,
        _grpc_port: u16,
        _ui_port: u16,
        _image_name: &str,
        _image_tag: &str,
        _container_name: &str,
    ) {
        self.grpc_endpoint = Some("127.0.0.1:7233".to_string());
        self.ui_url = Some("http://127.0.0.1:8233".to_string());
        self.events.lock().unwrap().push(Event::TemporalStart);
    }

    async fn stop(&mut self) {
        self.grpc_endpoint = None;
        self.ui_url = None;
        self.events.lock().unwrap().push(Event::TemporalStop);
    }

    fn grpc_endpoint(&self) -> Option<&str> {
        self.grpc_endpoint.as_deref()
    }

    fn ui_url(&self) -> Option<&str> {
        self.ui_url.as_deref()
    }
}

struct CountingReadinessCheck {
    calls: Arc<Mutex<u32>>,
}

#[async_trait]
impl ReadinessCheck for CountingReadinessCheck {
    async fn is_ready(
        &self,
        _identifier: &str,
        _grpc_endpoint: &str,
        _timeout_ms: u64,
    ) -> Result<(), String> {
        *self.calls.lock().unwrap() += 1;
        Ok(())
    }
}

fn build_temporal(
    events: Arc<Mutex<Vec<Event>>>,
    calls: Arc<Mutex<u32>>,
) -> TemporalDependency {
    TemporalDependency::builder("temporal-lifecycle")
        .with_impl(FakeTemporalImpl {
            grpc_endpoint: None,
            ui_url: None,
            events,
        })
        .with_readiness_check(CountingReadinessCheck { calls })
        .build()
}

#[tokio::test]
async fn hard_reset_running_dep_restarts_impl_and_rechecks_readiness() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let calls = Arc::new(Mutex::new(0u32));
    let mut dep = build_temporal(events.clone(), calls.clone());

    dep.start().await;
    dep.hard_reset().await;
    dep.stop().await;

    assert_eq!(
        events.lock().unwrap().as_slice(),
        &[Event::TemporalStart, Event::TemporalStop, Event::TemporalStart, Event::TemporalStop]
    );
    assert_eq!(*calls.lock().unwrap(), 2);
}

#[tokio::test]
async fn hard_reset_not_running_dep_is_noop() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let calls = Arc::new(Mutex::new(0u32));
    let mut dep = build_temporal(events.clone(), calls.clone());

    dep.hard_reset().await;

    assert!(events.lock().unwrap().is_empty());
    assert_eq!(*calls.lock().unwrap(), 0);
}

#[tokio::test]
async fn soft_reset_running_dep_does_not_restart_impl() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let calls = Arc::new(Mutex::new(0u32));
    let mut dep = build_temporal(events.clone(), calls.clone());

    dep.start().await;
    dep.soft_reset().await;
    dep.stop().await;

    assert_eq!(
        events.lock().unwrap().as_slice(),
        &[Event::TemporalStart, Event::TemporalStop]
    );
}

#[tokio::test]
async fn soft_reset_not_running_dep_is_noop() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let calls = Arc::new(Mutex::new(0u32));
    let dep = build_temporal(events.clone(), calls.clone());

    dep.soft_reset().await;

    assert!(events.lock().unwrap().is_empty());
}

#[tokio::test]
async fn add_child_before_start_starts_and_stops_child() {
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

        async fn start(&mut self) {
            self.events.lock().unwrap().push(ChildEvent::Start);
        }

        async fn stop(&mut self) {
            self.events.lock().unwrap().push(ChildEvent::Stop);
        }

        async fn soft_reset(&self) {}

        async fn hard_reset(&mut self) {}

        fn add_child(&mut self, _dep: Box<dyn RunnableDependency>) {}
    fn children(&self) -> &[Dependency] {
        &[]
    }
    fn children_mut(&mut self) -> &mut [Dependency] {
        &mut []
    }
    }

    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let calls = Arc::new(Mutex::new(0u32));
    let mut dep = build_temporal(events.clone(), calls.clone());

    let child_events = Arc::new(Mutex::new(Vec::<ChildEvent>::new()));
    dep.add_child(Box::new(RecordingChildDependency {
        events: child_events.clone(),
    }));

    dep.start().await;
    dep.stop().await;

    assert_eq!(
        child_events.lock().unwrap().as_slice(),
        &[ChildEvent::Start, ChildEvent::Stop]
    );
}

#[tokio::test]
async fn grpc_endpoint_and_ui_url_absent_before_start_present_after() {
    let events = Arc::new(Mutex::new(Vec::<Event>::new()));
    let calls = Arc::new(Mutex::new(0u32));
    let mut dep = build_temporal(events.clone(), calls.clone());

    assert_eq!(dep.grpc_endpoint(), None);
    assert_eq!(dep.ui_url(), None);

    dep.start().await;

    assert_eq!(dep.grpc_endpoint(), Some("127.0.0.1:7233"));
    assert_eq!(dep.ui_url(), Some("http://127.0.0.1:8233"));

    dep.stop().await;
}

async fn spawn_real_grpc_health_server() -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (_reporter, health_service) = tonic_health::server::health_reporter();

    tokio::spawn(async move {
        tonic::transport::Server::builder()
            .add_service(health_service)
            .serve_with_incoming(TcpListenerStream::new(listener))
            .await
    });

    addr
}

struct RealAddrTemporalImpl {
    grpc_endpoint: String,
}

#[async_trait]
impl TemporalImpl for RealAddrTemporalImpl {
    async fn start(
        &mut self,
        _grpc_port: u16,
        _ui_port: u16,
        _image_name: &str,
        _image_tag: &str,
        _container_name: &str,
    ) {
    }

    async fn stop(&mut self) {}

    fn grpc_endpoint(&self) -> Option<&str> {
        Some(&self.grpc_endpoint)
    }

    fn ui_url(&self) -> Option<&str> {
        None
    }
}

#[tokio::test]
async fn start_against_real_grpc_health_server_passes_default_readiness_check() {
    let addr = spawn_real_grpc_health_server().await;

    let mut dep = TemporalDependency::builder("temporal-real-readiness")
        .with_impl(RealAddrTemporalImpl {
            grpc_endpoint: addr.to_string(),
        })
        .build();

    dep.start().await;
    dep.stop().await;
}

#[tokio::test]
async fn stop_without_start_on_real_default_impl_does_not_panic() {
    let mut dep = TemporalDependency::builder("temporal-real-default-impl").build();

    assert_eq!(dep.grpc_endpoint(), None);
    assert_eq!(dep.ui_url(), None);

    dep.stop().await;

    assert_eq!(dep.grpc_endpoint(), None);
    assert_eq!(dep.ui_url(), None);
}
