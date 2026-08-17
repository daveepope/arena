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
