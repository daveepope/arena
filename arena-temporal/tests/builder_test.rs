use arena::dependency::RunnableDependency;
use arena::healthcheck::ReadinessCheck;
use arena_temporal::{TemporalDependency, TemporalImpl};
use async_trait::async_trait;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Default)]
struct StartArgs {
    grpc_port: u16,
    ui_port: u16,
    image_name: String,
    image_tag: String,
    container_name: String,
}

struct RecordingTemporalImpl {
    recorded: Arc<Mutex<Option<StartArgs>>>,
    grpc_endpoint: Option<String>,
    ui_url: Option<String>,
}

#[async_trait]
impl TemporalImpl for RecordingTemporalImpl {
    async fn start(
        &mut self,
        grpc_port: u16,
        ui_port: u16,
        image_name: &str,
        image_tag: &str,
        container_name: &str,
    ) {
        *self.recorded.lock().unwrap() = Some(StartArgs {
            grpc_port,
            ui_port,
            image_name: image_name.to_string(),
            image_tag: image_tag.to_string(),
            container_name: container_name.to_string(),
        });
        self.grpc_endpoint = Some("127.0.0.1:7233".to_string());
        self.ui_url = Some("http://127.0.0.1:8233".to_string());
    }

    async fn stop(&mut self) {
        self.grpc_endpoint = None;
        self.ui_url = None;
    }

    fn grpc_endpoint(&self) -> Option<&str> {
        self.grpc_endpoint.as_deref()
    }

    fn ui_url(&self) -> Option<&str> {
        self.ui_url.as_deref()
    }
}

struct OkReadinessCheck;

#[async_trait]
impl ReadinessCheck for OkReadinessCheck {
    async fn is_ready(
        &self,
        _identifier: &str,
        _grpc_endpoint: &str,
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

    async fn start(&mut self) {
        self.events.lock().unwrap().push(ChildEvent::Start);
    }

    async fn stop(&mut self) {
        self.events.lock().unwrap().push(ChildEvent::Stop);
    }

    async fn soft_reset(&self) {}

    async fn hard_reset(&mut self) {}

    fn add_child(&mut self, _dep: Box<dyn RunnableDependency>) {}
}

#[tokio::test]
async fn with_port_and_ui_port_propagate_to_impl_start() {
    let recorded = Arc::new(Mutex::new(None::<StartArgs>));
    let mut dep = TemporalDependency::builder("builder-ports")
        .with_port(11111)
        .with_ui_port(22222)
        .with_impl(RecordingTemporalImpl {
            recorded: recorded.clone(),
            grpc_endpoint: None,
            ui_url: None,
        })
        .with_readiness_check(OkReadinessCheck)
        .build();

    dep.start().await;
    dep.stop().await;

    let args = recorded.lock().unwrap().clone().expect("start should have recorded args");
    assert_eq!(args.grpc_port, 11111);
    assert_eq!(args.ui_port, 22222);
}

#[tokio::test]
async fn with_image_name_and_tag_propagate_to_impl_start() {
    let recorded = Arc::new(Mutex::new(None::<StartArgs>));
    let mut dep = TemporalDependency::builder("builder-image")
        .with_image_name("example.com/custom-temporal")
        .with_image_tag("9.9.9")
        .with_impl(RecordingTemporalImpl {
            recorded: recorded.clone(),
            grpc_endpoint: None,
            ui_url: None,
        })
        .with_readiness_check(OkReadinessCheck)
        .build();

    dep.start().await;
    dep.stop().await;

    let args = recorded.lock().unwrap().clone().expect("start should have recorded args");
    assert_eq!(args.image_name, "example.com/custom-temporal");
    assert_eq!(args.image_tag, "9.9.9");
}

#[tokio::test]
async fn with_image_alias_sets_same_tag_as_with_image_tag() {
    let recorded = Arc::new(Mutex::new(None::<StartArgs>));
    let mut dep = TemporalDependency::builder("builder-image-alias")
        .with_image("1.8.0")
        .with_impl(RecordingTemporalImpl {
            recorded: recorded.clone(),
            grpc_endpoint: None,
            ui_url: None,
        })
        .with_readiness_check(OkReadinessCheck)
        .build();

    dep.start().await;
    dep.stop().await;

    let args = recorded.lock().unwrap().clone().expect("start should have recorded args");
    assert_eq!(args.image_tag, "1.8.0");
}

#[tokio::test]
async fn with_container_tag_alias_sets_same_tag_as_with_image_tag() {
    let recorded = Arc::new(Mutex::new(None::<StartArgs>));
    let mut dep = TemporalDependency::builder("builder-container-tag-alias")
        .with_container_tag("2.0.0")
        .with_impl(RecordingTemporalImpl {
            recorded: recorded.clone(),
            grpc_endpoint: None,
            ui_url: None,
        })
        .with_readiness_check(OkReadinessCheck)
        .build();

    dep.start().await;
    dep.stop().await;

    let args = recorded.lock().unwrap().clone().expect("start should have recorded args");
    assert_eq!(args.image_tag, "2.0.0");
}

#[tokio::test]
async fn with_container_name_propagates_to_impl_start() {
    let recorded = Arc::new(Mutex::new(None::<StartArgs>));
    let mut dep = TemporalDependency::builder("builder-container-name")
        .with_container_name("my-temporal-container")
        .with_impl(RecordingTemporalImpl {
            recorded: recorded.clone(),
            grpc_endpoint: None,
            ui_url: None,
        })
        .with_readiness_check(OkReadinessCheck)
        .build();

    dep.start().await;
    dep.stop().await;

    let args = recorded.lock().unwrap().clone().expect("start should have recorded args");
    assert_eq!(args.container_name, "my-temporal-container");
}

#[tokio::test]
async fn with_child_dependencies_starts_and_stops_children() {
    let events = Arc::new(Mutex::new(Vec::<ChildEvent>::new()));
    let mut dep = TemporalDependency::builder("builder-children")
        .with_child_dependencies(vec![Box::new(RecordingChildDependency {
            events: events.clone(),
        })])
        .with_impl(RecordingTemporalImpl {
            recorded: Arc::new(Mutex::new(None)),
            grpc_endpoint: None,
            ui_url: None,
        })
        .with_readiness_check(OkReadinessCheck)
        .build();

    dep.start().await;
    dep.stop().await;

    assert_eq!(
        events.lock().unwrap().as_slice(),
        &[ChildEvent::Start, ChildEvent::Stop]
    );
}
