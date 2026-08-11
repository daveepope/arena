mod common;

use arena::component::RunnableComponent;
use arena_containerized_component::containerized_component::ContainerizedComponent;
use bollard::Docker;
use common::{find_container_mounts, unique_tag};

const CONTAINERFILE: &str = "FROM alpine:3.19\nCMD [\"sleep\", \"5\"]\n";

#[tokio::test]
async fn with_bind_mount_configures_container_host_config_mount() {
    let host_dir = std::env::temp_dir().join(unique_tag("arena-bind-mount-test-dir"));
    std::fs::create_dir_all(&host_dir).expect("create host bind mount dir");

    let image_tag = unique_tag("arena-bind-mount-test");
    let mut component = ContainerizedComponent::builder("bind-mount-test", CONTAINERFILE)
        .with_image_tag(&image_tag)
        .with_bind_mount(host_dir.to_str().expect("host dir is valid utf8"), "/mnt/data", false)
        .build()
        .await;

    component.start().await;

    let docker = Docker::connect_with_local_defaults().expect("connect to container runtime");
    let mounts = find_container_mounts(&docker, &image_tag).await;

    let mount = mounts
        .iter()
        .find(|m| m.destination.as_deref() == Some("/mnt/data"))
        .expect("bind mount destination should be configured on the container");
    assert_eq!(mount.rw, Some(true));

    component.stop().await;
    let _ = std::fs::remove_dir_all(&host_dir);
}

#[tokio::test]
async fn with_bind_mount_read_only_configures_read_only_container_mount() {
    let host_dir = std::env::temp_dir().join(unique_tag("arena-bind-mount-ro-test-dir"));
    std::fs::create_dir_all(&host_dir).expect("create host bind mount dir");

    let image_tag = unique_tag("arena-bind-mount-ro-test");
    let mut component = ContainerizedComponent::builder("bind-mount-ro-test", CONTAINERFILE)
        .with_image_tag(&image_tag)
        .with_bind_mount(host_dir.to_str().expect("host dir is valid utf8"), "/mnt/data", true)
        .build()
        .await;

    component.start().await;

    let docker = Docker::connect_with_local_defaults().expect("connect to container runtime");
    let mounts = find_container_mounts(&docker, &image_tag).await;

    let mount = mounts
        .iter()
        .find(|m| m.destination.as_deref() == Some("/mnt/data"))
        .expect("bind mount destination should be configured on the container");
    assert_eq!(mount.rw, Some(false));

    component.stop().await;
    let _ = std::fs::remove_dir_all(&host_dir);
}
