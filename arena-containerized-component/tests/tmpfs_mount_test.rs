mod common;

use arena::component::RunnableComponent;
use arena_containerized_component::containerized_component::ContainerizedComponent;
use bollard::Docker;
use common::{find_container_mounts, unique_tag};

const CONTAINERFILE: &str = "FROM alpine:3.19\nCMD [\"sleep\", \"5\"]\n";

#[tokio::test]
async fn with_tmpfs_mount_configures_tmpfs_on_container() {
    let image_tag = unique_tag("arena-tmpfs-mount-test");
    let mut component = ContainerizedComponent::builder("tmpfs-mount-test", CONTAINERFILE)
        .with_image_tag(&image_tag)
        .with_tmpfs_mount("/mnt/scratch", Some(16 * 1024 * 1024))
        .build()
        .await;

    component.start().await;

    let docker = Docker::connect_with_local_defaults().expect("connect to container runtime");
    let mounts = find_container_mounts(&docker, &image_tag).await;

    let mount = mounts
        .iter()
        .find(|m| m.destination.as_deref() == Some("/mnt/scratch"))
        .expect("tmpfs mount destination should be configured on the container");
    assert_eq!(mount.typ, Some(bollard::models::MountPointTypeEnum::TMPFS));

    component.stop().await;
}
