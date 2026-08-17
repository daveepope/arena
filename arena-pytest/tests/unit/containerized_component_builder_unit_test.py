from arena_pytest.exec.containerized_component import ContainerizedComponentBuilder


def test_build_minimal_name_and_containerfile_serializes_empty_volume_mappings():
    config = ContainerizedComponentBuilder("web", "FROM alpine:3.20").build()._for_ffi()
    assert config["volume_mappings"] == []


def test_with_volume_mapping_single_mapping_appends_host_and_container_path():
    config = (
        ContainerizedComponentBuilder("web", "FROM alpine:3.20")
        .with_volume_mapping("/host/one", "/container/one")
        .build()
        ._for_ffi()
    )
    assert config["volume_mappings"] == [
        {"host_path": "/host/one", "container_path": "/container/one"}
    ]


def test_with_volume_mapping_multiple_mappings_append_in_order():
    config = (
        ContainerizedComponentBuilder("web", "FROM alpine:3.20")
        .with_volume_mapping("/host/one", "/container/one")
        .with_volume_mapping("/host/two", "/container/two")
        .build()
        ._for_ffi()
    )
    assert config["volume_mappings"] == [
        {"host_path": "/host/one", "container_path": "/container/one"},
        {"host_path": "/host/two", "container_path": "/container/two"},
    ]
