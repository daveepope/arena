use arena::component::RunnableComponent;
use arena::healthcheck::ReadinessCheck;
use arena_containerized_component::containerized_component::{
    ContainerizedComponent, ContainerizedComponentImpl,
};
use arena_container::mount::{MountSpec, MountType};
use async_trait::async_trait;
use std::path::Path;
use std::sync::{Arc, Mutex};

const CONTAINERFILE: &str = "FROM alpine:3.19\n";

fn mount_type_str(mount_type: &MountType) -> &'static str {
    match mount_type {
        MountType::Bind => "bind",
        MountType::Volume => "volume",
        MountType::Tmpfs => "tmpfs",
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Event {
    BuildImage {
        identifier: String,
        image_tag: String,
        build_context: Option<String>,
    },
    StartContainer {
        identifier: String,
        network: Option<String>,
        network_alias: Option<String>,
        env_vars: Vec<(String, String)>,
        runtime_args: Vec<(String, String)>,
        port_mappings: Vec<(u16, u16)>,
        host_mappings: Vec<String>,
        mounts: Vec<(String, String, bool, Option<i64>)>,
    },
    FollowLogs {
        container_id: String,
    },
    StopContainer {
        container_id: String,
    },
}

struct FakeContainerizedComponentImpl {
    events: Arc<Mutex<Vec<Event>>>,
    next_container_id: String,
    order: Option<Arc<Mutex<Vec<String>>>>,
}

#[async_trait]
impl ContainerizedComponentImpl for FakeContainerizedComponentImpl {
    async fn build_image(
        &self,
        identifier: &str,
        _containerfile: &str,
        image_tag: &str,
        build_context: Option<&Path>,
    ) {
        self.events.lock().unwrap().push(Event::BuildImage {
            identifier: identifier.to_string(),
            image_tag: image_tag.to_string(),
            build_context: build_context.map(|p| p.to_string_lossy().into_owned()),
        });
    }

    #[allow(clippy::too_many_arguments)]
    async fn start_container(
        &self,
        identifier: &str,
        _image_tag: &str,
        network: Option<&str>,
        network_alias: Option<&str>,
        env_vars: &[(String, String)],
        runtime_args: &[(String, String)],
        port_mappings: &[(u16, u16)],
        host_mappings: &[String],
        mounts: &[MountSpec],
    ) -> String {
        self.events.lock().unwrap().push(Event::StartContainer {
            identifier: identifier.to_string(),
            network: network.map(|n| n.to_string()),
            network_alias: network_alias.map(|a| a.to_string()),
            env_vars: env_vars.to_vec(),
            runtime_args: runtime_args.to_vec(),
            port_mappings: port_mappings.to_vec(),
            host_mappings: host_mappings.to_vec(),
            mounts: mounts
                .iter()
                .map(|m| {
                    (
                        mount_type_str(&m.mount_type).to_string(),
                        m.container_path.clone(),
                        m.read_only,
                        m.tmpfs_size_bytes,
                    )
                })
                .collect(),
        });
        if let Some(order) = &self.order {
            order.lock().unwrap().push("start_container".to_string());
        }
        self.next_container_id.clone()
    }

    fn follow_logs(&self, container_id: &str, _identifier: &str) {
        self.events.lock().unwrap().push(Event::FollowLogs {
            container_id: container_id.to_string(),
        });
        if let Some(order) = &self.order {
            order.lock().unwrap().push("follow_logs".to_string());
        }
    }

    async fn stop_container(&self, container_id: &str, _identifier: &str) {
        self.events.lock().unwrap().push(Event::StopContainer {
            container_id: container_id.to_string(),
        });
        if let Some(order) = &self.order {
            order.lock().unwrap().push("stop_container".to_string());
        }
    }
}

fn setup_fake() -> (FakeContainerizedComponentImpl, Arc<Mutex<Vec<Event>>>) {
    let events = Arc::new(Mutex::new(Vec::new()));
    let fake = FakeContainerizedComponentImpl {
        events: events.clone(),
        next_container_id: "container-123".to_string(),
        order: None,
    };
    (fake, events)
}

struct FakeReadinessCheck {
    order: Arc<Mutex<Vec<String>>>,
    last_identifier: Arc<Mutex<Option<String>>>,
    last_target: Arc<Mutex<Option<String>>>,
    last_timeout_ms: Arc<Mutex<Option<u64>>>,
}

#[async_trait]
impl ReadinessCheck for FakeReadinessCheck {
    async fn is_ready(
        &self,
        identifier: &str,
        target: &str,
        timeout_ms: u64,
    ) -> Result<(), String> {
        self.order.lock().unwrap().push("readiness_check".to_string());
        *self.last_identifier.lock().unwrap() = Some(identifier.to_string());
        *self.last_target.lock().unwrap() = Some(target.to_string());
        *self.last_timeout_ms.lock().unwrap() = Some(timeout_ms);
        Ok(())
    }
}

struct FailingReadinessCheck;

#[async_trait]
impl ReadinessCheck for FailingReadinessCheck {
    async fn is_ready(&self, _identifier: &str, _target: &str, _timeout_ms: u64) -> Result<(), String> {
        Err("readiness never came up".to_string())
    }
}

struct FakeComponent {
    identifier: String,
    order: Arc<Mutex<Vec<String>>>,
}

#[async_trait]
impl RunnableComponent for FakeComponent {
    async fn start(&mut self) {
        self.order
            .lock()
            .unwrap()
            .push(format!("{}-start", self.identifier));
    }

    async fn stop(&mut self) {
        self.order
            .lock()
            .unwrap()
            .push(format!("{}-stop", self.identifier));
    }

    fn add_child(&mut self, _child: Box<dyn RunnableComponent>) {}
}

#[tokio::test]
async fn build_no_image_tag_sanitizes_identifier_for_tag() {
    let (fake, events) = setup_fake();
    let _component = ContainerizedComponent::builder("build-default-tag", CONTAINERFILE)
        .with_impl(fake)
        .build()
        .await;

    let events = events.lock().unwrap();
    match events.first() {
        Some(Event::BuildImage { image_tag, .. }) => {
            assert!(image_tag.contains("build-default-tag"))
        }
        other => panic!("expected BuildImage event, got {:?}", other),
    }
}

#[tokio::test]
async fn build_with_image_tag_uses_provided_tag() {
    let (fake, events) = setup_fake();
    let _component = ContainerizedComponent::builder("build-explicit-tag", CONTAINERFILE)
        .with_impl(fake)
        .with_image_tag("custom-tag")
        .build()
        .await;

    let events = events.lock().unwrap();
    match events.first() {
        Some(Event::BuildImage { image_tag, .. }) => assert_eq!(image_tag, "custom-tag"),
        other => panic!("expected BuildImage event, got {:?}", other),
    }
}

#[tokio::test]
async fn build_with_build_context_resolves_and_passes_path() {
    let (fake, events) = setup_fake();
    let tmp_dir = std::env::temp_dir();
    let _component = ContainerizedComponent::builder("build-context-test", CONTAINERFILE)
        .with_impl(fake)
        .with_build_context(tmp_dir.to_str().expect("tmp dir is valid utf8"))
        .build()
        .await;

    let events = events.lock().unwrap();
    match events.first() {
        Some(Event::BuildImage { build_context, .. }) => {
            assert_eq!(build_context.as_deref(), tmp_dir.to_str());
        }
        other => panic!("expected BuildImage event, got {:?}", other),
    }
}

#[tokio::test]
async fn start_calls_start_container_then_follow_logs_in_order() {
    let (fake, events) = setup_fake();
    let mut component = ContainerizedComponent::builder("start-order-test", CONTAINERFILE)
        .with_impl(fake)
        .build()
        .await;

    component.start().await;

    let events = events.lock().unwrap();
    let kinds: Vec<&str> = events
        .iter()
        .map(|e| match e {
            Event::BuildImage { .. } => "build_image",
            Event::StartContainer { .. } => "start_container",
            Event::FollowLogs { .. } => "follow_logs",
            Event::StopContainer { .. } => "stop_container",
        })
        .collect();
    assert_eq!(kinds, vec!["build_image", "start_container", "follow_logs"]);
}

#[tokio::test]
async fn start_passes_network_and_port_mappings_to_impl() {
    let (fake, events) = setup_fake();
    let mut component = ContainerizedComponent::builder("start-args-test", CONTAINERFILE)
        .with_impl(fake)
        .with_network("test-net")
        .with_port_mapping(8080, 80)
        .build()
        .await;

    component.start().await;

    let events = events.lock().unwrap();
    let (network, port_mappings) = events
        .iter()
        .find_map(|e| match e {
            Event::StartContainer {
                network,
                port_mappings,
                ..
            } => Some((network.clone(), port_mappings.clone())),
            _ => None,
        })
        .expect("start_container event should be recorded");

    assert_eq!(network, Some("test-net".to_string()));
    assert_eq!(port_mappings, vec![(8080, 80)]);
}

#[tokio::test]
async fn start_passes_network_alias_env_vars_runtime_args_and_host_mappings_to_impl() {
    let (fake, events) = setup_fake();
    let mut component = ContainerizedComponent::builder("start-config-test", CONTAINERFILE)
        .with_impl(fake)
        .with_network("test-net")
        .with_network_alias("test-alias")
        .with_env_var("KEY", "value")
        .with_runtime_arg("--flag", "on")
        .with_host_mapping("host.local:127.0.0.1")
        .build()
        .await;

    component.start().await;

    let events = events.lock().unwrap();
    let event = events
        .iter()
        .find_map(|e| match e {
            Event::StartContainer {
                network_alias,
                env_vars,
                runtime_args,
                host_mappings,
                ..
            } => Some((
                network_alias.clone(),
                env_vars.clone(),
                runtime_args.clone(),
                host_mappings.clone(),
            )),
            _ => None,
        })
        .expect("start_container event should be recorded");

    assert_eq!(event.0, Some("test-alias".to_string()));
    assert_eq!(event.1, vec![("KEY".to_string(), "value".to_string())]);
    assert_eq!(event.2, vec![("--flag".to_string(), "on".to_string())]);
    assert_eq!(event.3, vec!["host.local:127.0.0.1".to_string()]);
}

#[tokio::test]
async fn start_passes_resolved_bind_mount_to_impl() {
    let (fake, events) = setup_fake();
    let tmp_dir = std::env::temp_dir();
    let mut component = ContainerizedComponent::builder("start-bind-mount-test", CONTAINERFILE)
        .with_impl(fake)
        .with_bind_mount(
            tmp_dir.to_str().expect("tmp dir is valid utf8"),
            "/mnt/data",
            true,
        )
        .build()
        .await;

    component.start().await;

    let events = events.lock().unwrap();
    let mounts = events
        .iter()
        .find_map(|e| match e {
            Event::StartContainer { mounts, .. } => Some(mounts.clone()),
            _ => None,
        })
        .expect("start_container event should be recorded");

    assert_eq!(
        mounts,
        vec![("bind".to_string(), "/mnt/data".to_string(), true, None)]
    );
}

#[tokio::test]
async fn start_passes_volume_mount_to_impl() {
    let (fake, events) = setup_fake();
    let mut component = ContainerizedComponent::builder("start-volume-mount-test", CONTAINERFILE)
        .with_impl(fake)
        .with_volume_mount("my-volume", "/mnt/data", false)
        .build()
        .await;

    component.start().await;

    let events = events.lock().unwrap();
    let mounts = events
        .iter()
        .find_map(|e| match e {
            Event::StartContainer { mounts, .. } => Some(mounts.clone()),
            _ => None,
        })
        .expect("start_container event should be recorded");

    assert_eq!(
        mounts,
        vec![("volume".to_string(), "/mnt/data".to_string(), false, None)]
    );
}

#[tokio::test]
async fn start_passes_tmpfs_mount_to_impl() {
    let (fake, events) = setup_fake();
    let mut component = ContainerizedComponent::builder("start-tmpfs-mount-test", CONTAINERFILE)
        .with_impl(fake)
        .with_tmpfs_mount("/mnt/scratch", Some(16 * 1024 * 1024))
        .build()
        .await;

    component.start().await;

    let events = events.lock().unwrap();
    let mounts = events
        .iter()
        .find_map(|e| match e {
            Event::StartContainer { mounts, .. } => Some(mounts.clone()),
            _ => None,
        })
        .expect("start_container event should be recorded");

    assert_eq!(
        mounts,
        vec![(
            "tmpfs".to_string(),
            "/mnt/scratch".to_string(),
            false,
            Some(16 * 1024 * 1024)
        )]
    );
}

#[tokio::test]
async fn stop_stops_container_using_returned_container_id() {
    let (fake, events) = setup_fake();
    let mut component = ContainerizedComponent::builder("stop-test", CONTAINERFILE)
        .with_impl(fake)
        .build()
        .await;

    component.start().await;
    component.stop().await;

    let events = events.lock().unwrap();
    match events.last() {
        Some(Event::StopContainer { container_id }) => assert_eq!(container_id, "container-123"),
        other => panic!("expected StopContainer event, got {:?}", other),
    }
}

#[tokio::test]
async fn stop_without_start_does_not_call_stop_container() {
    let (fake, events) = setup_fake();
    let mut component = ContainerizedComponent::builder("stop-without-start-test", CONTAINERFILE)
        .with_impl(fake)
        .build()
        .await;

    component.stop().await;

    let events = events.lock().unwrap();
    assert!(!events.iter().any(|e| matches!(e, Event::StopContainer { .. })));
}

#[tokio::test]
async fn start_runs_readiness_check_after_container_starts() {
    let order = Arc::new(Mutex::new(Vec::new()));
    let (mut fake, _events) = setup_fake();
    fake.order = Some(order.clone());

    let last_identifier = Arc::new(Mutex::new(None));
    let last_target = Arc::new(Mutex::new(None));
    let last_timeout_ms = Arc::new(Mutex::new(None));
    let check = FakeReadinessCheck {
        order: order.clone(),
        last_identifier: last_identifier.clone(),
        last_target: last_target.clone(),
        last_timeout_ms: last_timeout_ms.clone(),
    };

    let mut component = ContainerizedComponent::builder("readiness-order-test", CONTAINERFILE)
        .with_impl(fake)
        .with_readiness_check(check, "readiness-target")
        .build()
        .await;

    component.start().await;

    assert_eq!(
        *order.lock().unwrap(),
        vec![
            "start_container".to_string(),
            "follow_logs".to_string(),
            "readiness_check".to_string(),
        ]
    );
    assert_eq!(*last_target.lock().unwrap(), Some("readiness-target".to_string()));
    assert_eq!(*last_timeout_ms.lock().unwrap(), Some(10_000));
    assert!(last_identifier
        .lock()
        .unwrap()
        .as_deref()
        .expect("identifier should be recorded")
        .contains("readiness-order-test"));
}

#[tokio::test]
async fn with_readiness_check_timeout_passes_custom_timeout_to_check() {
    let order = Arc::new(Mutex::new(Vec::new()));
    let (fake, _events) = setup_fake();

    let last_timeout_ms = Arc::new(Mutex::new(None));
    let check = FakeReadinessCheck {
        order: order.clone(),
        last_identifier: Arc::new(Mutex::new(None)),
        last_target: Arc::new(Mutex::new(None)),
        last_timeout_ms: last_timeout_ms.clone(),
    };

    let mut component = ContainerizedComponent::builder("readiness-timeout-test", CONTAINERFILE)
        .with_impl(fake)
        .with_readiness_check_timeout(check, "readiness-target", 5_000)
        .build()
        .await;

    component.start().await;

    assert_eq!(*last_timeout_ms.lock().unwrap(), Some(5_000));
}

#[tokio::test]
#[should_panic(expected = "readiness check failed for target readiness-target")]
async fn start_with_failing_readiness_check_panics() {
    let (fake, _events) = setup_fake();

    let mut component = ContainerizedComponent::builder("readiness-failure-test", CONTAINERFILE)
        .with_impl(fake)
        .with_readiness_check(FailingReadinessCheck, "readiness-target")
        .build()
        .await;

    component.start().await;
}

#[tokio::test]
async fn add_child_starts_before_and_stops_after_container() {
    let order = Arc::new(Mutex::new(Vec::new()));
    let (mut fake, _events) = setup_fake();
    fake.order = Some(order.clone());

    let mut component = ContainerizedComponent::builder("add-child-test", CONTAINERFILE)
        .with_impl(fake)
        .build()
        .await;

    component.add_child(Box::new(FakeComponent {
        identifier: "child".to_string(),
        order: order.clone(),
    }));

    component.start().await;
    component.stop().await;

    assert_eq!(
        *order.lock().unwrap(),
        vec![
            "child-start".to_string(),
            "start_container".to_string(),
            "follow_logs".to_string(),
            "stop_container".to_string(),
            "child-stop".to_string(),
        ]
    );
}

#[tokio::test]
async fn with_child_components_starts_children_before_container() {
    let order = Arc::new(Mutex::new(Vec::new()));
    let (mut fake, _events) = setup_fake();
    fake.order = Some(order.clone());

    let child: arena::Component = Box::new(FakeComponent {
        identifier: "pre-wired-child".to_string(),
        order: order.clone(),
    });

    let mut component = ContainerizedComponent::builder("with-child-components-test", CONTAINERFILE)
        .with_impl(fake)
        .with_child_components(vec![child])
        .build()
        .await;

    component.start().await;
    component.stop().await;

    assert_eq!(
        *order.lock().unwrap(),
        vec![
            "pre-wired-child-start".to_string(),
            "start_container".to_string(),
            "follow_logs".to_string(),
            "stop_container".to_string(),
            "pre-wired-child-stop".to_string(),
        ]
    );
}
