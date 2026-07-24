from arena_pytest.dep.smtp import SmtpDependencyBuilder


def test_build_minimal_name_serializes_type_and_identifier():
    config = SmtpDependencyBuilder("smtp").build()._for_ffi()
    assert config["type"] == "smtp"
    assert config["identifier"].startswith("arena-smtp-smtp-")
    assert "image" not in config
    assert "port" not in config
    assert "starttls" not in config


def test_with_starttls_sets_flag():
    config = SmtpDependencyBuilder("smtp").with_starttls().build()._for_ffi()
    assert config["starttls"] is True


def test_build_with_overrides_serializes_configured_fields():
    config = (
        SmtpDependencyBuilder("smtp")
        .with_image("v1.30.5")
        .with_image_name("axllent/mailpit")
        .with_port(11025)
        .with_ui_port(18025)
        .with_container_name("smtp-box")
        .build()
        ._for_ffi()
    )
    assert config["image"] == "v1.30.5"
    assert config["image_name"] == "axllent/mailpit"
    assert config["port"] == 11025
    assert config["ui_port"] == 18025
    assert config["container_name"] == "smtp-box"
