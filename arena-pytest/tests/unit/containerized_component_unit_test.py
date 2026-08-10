from arena_pytest.exec.containerized_component import ContainerizedComponentBuilder


def test_with_bind_mount_serializes_host_and_container_path():
    config = (
        ContainerizedComponentBuilder("test", "Dockerfile")
        .with_bind_mount("/host/data", "/mnt/data", read_only=True)
        .build()
        ._for_ffi()
    )
    assert len(config["bind_mounts"]) == 1
    assert config["bind_mounts"][0]["host_path"] == "/host/data"
    assert config["bind_mounts"][0]["container_path"] == "/mnt/data"
    assert config["bind_mounts"][0]["read_only"] is True


def test_with_bind_mount_no_read_only_arg_defaults_read_only_to_false():
    config = (
        ContainerizedComponentBuilder("test", "Dockerfile")
        .with_bind_mount("/host/data", "/mnt/data")
        .build()
        ._for_ffi()
    )
    assert config["bind_mounts"][0]["read_only"] is False


def test_without_bind_mount_serializes_empty_list():
    config = ContainerizedComponentBuilder("test", "Dockerfile").build()._for_ffi()
    assert config["bind_mounts"] == []
