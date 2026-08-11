from arena_pytest.exec.containerized_component import ContainerizedComponentBuilder


def test_with_bind_mount_serializes_source_and_container_path():
    config = (
        ContainerizedComponentBuilder("test", "Dockerfile")
        .with_bind_mount("/host/data", "/mnt/data", read_only=True)
        .build()
        ._for_ffi()
    )
    assert len(config["mounts"]) == 1
    assert config["mounts"][0]["type"] == "bind"
    assert config["mounts"][0]["source"] == "/host/data"
    assert config["mounts"][0]["container_path"] == "/mnt/data"
    assert config["mounts"][0]["read_only"] is True


def test_with_bind_mount_no_read_only_arg_defaults_read_only_to_false():
    config = (
        ContainerizedComponentBuilder("test", "Dockerfile")
        .with_bind_mount("/host/data", "/mnt/data")
        .build()
        ._for_ffi()
    )
    assert config["mounts"][0]["read_only"] is False


def test_with_volume_mount_serializes_volume_name_and_container_path():
    config = (
        ContainerizedComponentBuilder("test", "Dockerfile")
        .with_volume_mount("my-volume", "/mnt/data", read_only=True)
        .build()
        ._for_ffi()
    )
    assert len(config["mounts"]) == 1
    assert config["mounts"][0]["type"] == "volume"
    assert config["mounts"][0]["source"] == "my-volume"
    assert config["mounts"][0]["container_path"] == "/mnt/data"
    assert config["mounts"][0]["read_only"] is True


def test_with_tmpfs_mount_serializes_container_path_and_size_bytes():
    config = (
        ContainerizedComponentBuilder("test", "Dockerfile")
        .with_tmpfs_mount("/mnt/data", size_bytes=1024)
        .build()
        ._for_ffi()
    )
    assert len(config["mounts"]) == 1
    assert config["mounts"][0]["type"] == "tmpfs"
    assert config["mounts"][0]["container_path"] == "/mnt/data"
    assert config["mounts"][0]["size_bytes"] == 1024


def test_with_tmpfs_mount_no_size_bytes_arg_omits_size_bytes():
    config = (
        ContainerizedComponentBuilder("test", "Dockerfile")
        .with_tmpfs_mount("/mnt/data")
        .build()
        ._for_ffi()
    )
    assert "size_bytes" not in config["mounts"][0]


def test_without_mounts_serializes_empty_list():
    config = ContainerizedComponentBuilder("test", "Dockerfile").build()._for_ffi()
    assert config["mounts"] == []
