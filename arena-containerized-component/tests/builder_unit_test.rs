use arena_containerized_component::containerized_component::ContainerizedComponent;

#[test]
fn with_volume_mapping_single_mapping_chains_for_further_building() {
    let _builder = ContainerizedComponent::builder("probe", "FROM alpine:3.20")
        .with_volume_mapping("/host/path", "/container/path");
}

#[test]
fn with_volume_mapping_multiple_mappings_chain_for_further_building() {
    let _builder = ContainerizedComponent::builder("probe", "FROM alpine:3.20")
        .with_volume_mapping("/host/one", "/container/one")
        .with_volume_mapping("/host/two", "/container/two");
}

#[test]
fn from_image_chains_for_further_building() {
    let _builder = ContainerizedComponent::from_image("probe", "alpine:3.20")
        .with_port_mapping(8080, 80);
}

#[test]
fn with_platform_chains_for_further_building() {
    let _builder = ContainerizedComponent::builder("probe", "FROM alpine:3.20")
        .with_platform("linux/arm64");
}

#[test]
fn from_image_with_platform_chains_for_further_building() {
    let _builder = ContainerizedComponent::from_image("probe", "alpine:3.20")
        .with_platform("linux/amd64");
}

#[tokio::test]
#[should_panic(expected = "with_build_context has no effect when using from_image")]
async fn from_image_with_build_context_build_panics() {
    ContainerizedComponent::from_image("probe", "alpine:3.20")
        .with_build_context(".")
        .build()
        .await;
}

#[tokio::test]
#[should_panic(expected = "with_image_tag has no effect when using from_image")]
async fn from_image_with_image_tag_build_panics() {
    ContainerizedComponent::from_image("probe", "alpine:3.20")
        .with_image_tag("custom-tag")
        .build()
        .await;
}
