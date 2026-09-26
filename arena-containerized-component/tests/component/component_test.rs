use arena::component::RunnableComponent;
use arena_containerized_component::containerized_component::ContainerizedComponent;
use bollard::Docker;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

fn tcp_reachable(addr: SocketAddr) -> bool {
    TcpStream::connect_timeout(&addr, Duration::from_millis(200)).is_ok()
}

async fn wait_for_tcp_port(port: u16, timeout: Duration) -> bool {
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port);
    let deadline = Instant::now() + timeout;
    loop {
        if tcp_reachable(addr) {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

#[tokio::test]
async fn start_from_image_pulls_and_runs_prebuilt_image() {
    const HOST_PORT: u16 = 16379;

    let mut component = ContainerizedComponent::from_image("from-image-probe", "redis:8-alpine")
        .with_platform(arena_container::platform::docker_platform())
        .with_port_mapping(HOST_PORT, 6379)
        .build()
        .await
        .expect("build containerized component");

    component.start().await.expect("component should start");

    let reachable = wait_for_tcp_port(HOST_PORT, Duration::from_secs(15)).await;

    component.stop().await.expect("component should stop");

    assert!(
        reachable,
        "expected container started from a pre-built image to be reachable on its mapped port"
    );
}

const CONTAINERFILE: &str = r#"FROM alpine:3.20
CMD ["sh", "-c", "echo mounted-ok > /mnt/test/marker.txt && sleep 30"]
"#;

async fn ensure_base_image_pulled() {
    let docker = Docker::connect_with_local_defaults().expect("connect to container runtime");
    arena_container::image::pull_image(
        "volume-mapping-probe",
        "alpine:3.20",
        &arena_container::platform::docker_platform(),
        &docker,
    )
    .await
    .expect("pull base image");
}

fn unique_host_dir(name: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock")
        .as_nanos();
    let base = std::env::var_os("TEST_TMPDIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let dir = base.join(format!(
        "arena-containerized-component-{name}-{}-{nanos}",
        std::process::id(),
    ));
    std::fs::create_dir_all(&dir).expect("create host volume dir");
    dir
}

async fn wait_for_marker(path: &Path, timeout: Duration) -> Option<String> {
    let deadline = Instant::now() + timeout;
    loop {
        if let Ok(contents) = std::fs::read_to_string(path) {
            return Some(contents);
        }
        if Instant::now() >= deadline {
            return None;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

#[tokio::test]
async fn start_with_volume_mapping_writes_file_visible_on_host() {
    ensure_base_image_pulled().await;

    let host_dir = unique_host_dir("volume-mapping-probe");
    let marker_path = host_dir.join("marker.txt");

    let mut component = ContainerizedComponent::builder("volume-mapping-probe", CONTAINERFILE)
        .with_volume_mapping(host_dir.to_string_lossy().to_string(), "/mnt/test")
        .build()
        .await
        .expect("build containerized component");

    component.start().await.expect("component should start");

    let contents = wait_for_marker(&marker_path, Duration::from_secs(15)).await;

    component.stop().await.expect("component should stop");
    let _ = std::fs::remove_dir_all(&host_dir);

    assert_eq!(contents.as_deref(), Some("mounted-ok\n"));
}

async fn container_labels(container_name: &str) -> std::collections::HashMap<String, String> {
    let docker = Docker::connect_with_defaults().expect("connect to container runtime");
    docker
        .inspect_container(
            container_name,
            None::<bollard::query_parameters::InspectContainerOptions>,
        )
        .await
        .expect("container should be inspectable")
        .config
        .and_then(|config| config.labels)
        .unwrap_or_default()
}

#[tokio::test]
async fn start_default_expiry_stamps_expiry_labels_on_the_container() {
    let mut component = ContainerizedComponent::from_image("expiry-probe", "redis:8-alpine")
        .with_platform(arena_container::platform::docker_platform())
        .build()
        .await
        .expect("build containerized component");

    component.start().await.expect("component should start");
    let container_name =
        arena_container::identifier::sanitize_for_container(component.identifier());
    let labels = container_labels(&container_name).await;
    component.stop().await.expect("component should stop");

    assert_eq!(
        labels.get(arena_container::expiry::MODULE_LABEL).map(String::as_str),
        Some("arena-containerized-component")
    );
    assert!(labels.contains_key(arena_container::expiry::EXPIRES_AT_LABEL));
}

#[tokio::test]
async fn start_without_expiry_stamps_no_expiry_labels_on_the_container() {
    let mut component = ContainerizedComponent::from_image("no-expiry-probe", "redis:8-alpine")
        .with_platform(arena_container::platform::docker_platform())
        .without_expiry()
        .build()
        .await
        .expect("build containerized component");

    component.start().await.expect("component should start");
    let container_name =
        arena_container::identifier::sanitize_for_container(component.identifier());
    let labels = container_labels(&container_name).await;
    component.stop().await.expect("component should stop");

    assert!(!labels.contains_key(arena_container::expiry::MODULE_LABEL));
    assert!(!labels.contains_key(arena_container::expiry::EXPIRES_AT_LABEL));
}
