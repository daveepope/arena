mod common;

use arena::component::RunnableComponent;
use arena_containerized_component::containerized_component::ContainerizedComponent;
use bollard::Docker;
use common::{find_container_mounts, unique_tag};

const CONTAINERFILE: &str = "FROM alpine:3.19\nCMD [\"sleep\", \"5\"]\n";

#[tokio::test]
async fn with_volume_mount_configures_named_volume_on_container() {
    let volume_name = unique_tag("arena-volume-mount-test-vol");
    let image_tag = unique_tag("arena-volume-mount-test");
    let mut component = ContainerizedComponent::builder("volume-mount-test", CONTAINERFILE)
        .with_image_tag(&image_tag)
        .with_volume_mount(&volume_name, "/mnt/data", false)
        .build()
        .await;

    component.start().await;

    let docker = Docker::connect_with_local_defaults().expect("connect to container runtime");
    let mounts = find_container_mounts(&docker, &image_tag).await;

    let mount = mounts
        .iter()
        .find(|m| m.destination.as_deref() == Some("/mnt/data"))
        .expect("volume mount destination should be configured on the container");
    assert_eq!(mount.name.as_deref(), Some(volume_name.as_str()));
    assert_eq!(mount.rw, Some(true));

    component.stop().await;
    let _ = docker
        .remove_volume(
            &volume_name,
            None::<bollard::query_parameters::RemoveVolumeOptions>,
        )
        .await;
}
