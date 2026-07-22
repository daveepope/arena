from arena_pytest.dep.temporal import TemporalDependencyBuilder


def test_build_minimal_name_serializes_type_and_identifier():
    config = TemporalDependencyBuilder("temporal").build()._for_ffi()
    assert config["type"] == "temporal"
    assert config["identifier"].startswith("arena-temporal-temporal-")
    assert "image" not in config
    assert "port" not in config


def test_build_with_overrides_serializes_configured_fields():
    config = (
        TemporalDependencyBuilder("temporal")
        .with_image("1.24.2")
        .with_image_name("temporalio/auto-setup")
        .with_port(17233)
        .with_ui_port(18233)
        .with_container_name("temporal-box")
        .build()
        ._for_ffi()
    )
    assert config["image"] == "1.24.2"
    assert config["image_name"] == "temporalio/auto-setup"
    assert config["port"] == 17233
    assert config["ui_port"] == 18233
    assert config["container_name"] == "temporal-box"
