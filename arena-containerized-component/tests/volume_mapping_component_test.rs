use arena::component::RunnableComponent;
use arena_containerized_component::containerized_component::ContainerizedComponent;
use bollard::Docker;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

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

    component.start().await;

    let contents = wait_for_marker(&marker_path, Duration::from_secs(15)).await;

    component.stop().await;
    let _ = std::fs::remove_dir_all(&host_dir);

    assert_eq!(contents.as_deref(), Some("mounted-ok\n"));
}
