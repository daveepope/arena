use arena_containerized_component::containerized_component::ContainerizedComponent;

const CONTAINERFILE: &str = "FROM alpine:3.19\nCMD [\"sleep\", \"5\"]\n";

#[tokio::test]
#[should_panic(expected = "bind mount source path does not exist")]
async fn with_bind_mount_missing_absolute_source_panics() {
    let _ = ContainerizedComponent::builder("bind-mount-missing-abs", CONTAINERFILE)
        .with_bind_mount("/arena/bind/source/does/not/exist", "/mnt/data", false)
        .build()
        .await;
}

#[tokio::test]
#[should_panic(expected = "bind mount source path does not exist")]
async fn with_bind_mount_missing_relative_source_panics() {
    let _ = ContainerizedComponent::builder("bind-mount-missing-rel", CONTAINERFILE)
        .with_bind_mount("arena-bind-source-does-not-exist", "/mnt/data", false)
        .build()
        .await;
}
