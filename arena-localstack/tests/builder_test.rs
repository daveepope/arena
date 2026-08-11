use arena::dependency::RunnableDependency;
use arena_localstack::{
    EventRuleSpec, EventRuleTarget, EventTargetKind, LambdaSpec, LocalstackDependency,
};
use async_trait::async_trait;

struct FakeLocalstackImpl {
    endpoint: Option<String>,
}

#[async_trait]
impl arena_localstack::LocalstackImpl for FakeLocalstackImpl {
    async fn start(
        &mut self,
        _port: u16,
        _image_name: &str,
        _image_tag: &str,
        _container_name: &str,
        _services: &[String],
    ) {
        self.endpoint = Some("http://127.0.0.1:4566".to_string());
    }

    async fn stop(&mut self) {}

    fn endpoint_url(&self) -> Option<&str> {
        self.endpoint.as_deref()
    }
}

#[test]
fn with_image_name_and_tag_sets_defaults_via_impl() {
    let dep = LocalstackDependency::builder("localstack-image")
        .with_impl(FakeLocalstackImpl { endpoint: None })
        .with_image_name("custom/localstack")
        .with_image("2.3")
        .with_container_tag("2.4")
        .with_container_name("my-container")
        .with_network("my-network")
        .build();

    assert!(dep.identifier().contains("localstack-image"));
}

#[test]
fn with_service_and_services_accumulates_all() {
    let dep = LocalstackDependency::builder("localstack-services")
        .with_impl(FakeLocalstackImpl { endpoint: None })
        .with_service("sqs")
        .with_services(vec!["lambda", "events"])
        .build();

    assert!(dep.identifier().contains("localstack-services"));
}

#[test]
fn with_queue_and_fifo_queue_and_spec_registers_queues() {
    let dep = LocalstackDependency::builder("localstack-queues")
        .with_impl(FakeLocalstackImpl { endpoint: None })
        .with_queue("plain-queue")
        .with_fifo_queue("fifo-queue")
        .with_queue_spec(arena_localstack::QueueSpec {
            name: "spec-queue".to_string(),
            fifo: false,
        })
        .build();

    assert_eq!(dep.queue_url("plain-queue"), None);
    assert_eq!(dep.queue_url("fifo-queue"), None);
    assert_eq!(dep.queue_url("spec-queue"), None);
}

#[test]
fn with_lambda_registers_lambda_spec() {
    let dep = LocalstackDependency::builder("localstack-lambda")
        .with_impl(FakeLocalstackImpl { endpoint: None })
        .with_lambda(LambdaSpec {
            name: "my-fn".to_string(),
            runtime: "python3.12".to_string(),
            handler: "handler.main".to_string(),
            source_dir: std::path::PathBuf::from("/tmp/does-not-matter"),
            environment: vec![("KEY".to_string(), "VALUE".to_string())],
        })
        .build();

    assert_eq!(dep.lambda_arn("my-fn"), None);
}

#[test]
fn with_event_bus_and_rule_registers_event_resources() {
    let dep = LocalstackDependency::builder("localstack-events")
        .with_impl(FakeLocalstackImpl { endpoint: None })
        .with_event_bus("my-bus")
        .with_event_rule(EventRuleSpec {
            name: "my-rule".to_string(),
            event_bus: Some("my-bus".to_string()),
            event_pattern: "{}".to_string(),
            targets: vec![EventRuleTarget {
                target_id: "target-1".to_string(),
                kind: EventTargetKind::SqsQueue {
                    queue_name: "plain-queue".to_string(),
                },
            }],
        })
        .build();

    assert!(dep.identifier().contains("localstack-events"));
}

#[test]
fn build_without_impl_uses_default_container_impl() {
    let dep = LocalstackDependency::builder("localstack-default-impl").build();

    assert!(dep.identifier().contains("localstack-default-impl"));
    assert_eq!(dep.endpoint_url(), None);
}

#[test]
fn build_without_port_uses_auto_port() {
    let dep = LocalstackDependency::builder("localstack-default-port")
        .with_impl(FakeLocalstackImpl { endpoint: None })
        .build();

    assert!(dep.identifier().contains("localstack-default-port"));
}
