from arena_pytest.dep.oracle import OracleDependencyBuilder


def test_build_minimal_name_serializes_type_and_identifier():
    config = OracleDependencyBuilder("oracle").build()._for_ffi()
    assert config["type"] == "oracle"
    assert config["identifier"].startswith("arena-oracle-oracle-")
    assert "image" not in config
    assert "port" not in config
    assert "admin_password" not in config


def test_with_admin_password_sets_admin_password():
    config = OracleDependencyBuilder("oracle").with_admin_password("secret").build()._for_ffi()
    assert config["admin_password"] == "secret"


def test_build_with_overrides_serializes_configured_fields():
    config = (
        OracleDependencyBuilder("oracle")
        .with_image("23.5.0.24.07")
        .with_image_name("gvenzl/oracle-free")
        .with_port(11521)
        .with_database_name("orders")
        .with_database_username("orders_user")
        .with_database_password("orders_pw")
        .with_admin_password("admin_pw")
        .with_container_name("oracle-box")
        .with_startup_sql_scripts(["init.sql"])
        .build()
        ._for_ffi()
    )
    assert config["image"] == "23.5.0.24.07"
    assert config["image_name"] == "gvenzl/oracle-free"
    assert config["port"] == 11521
    assert config["database_name"] == "orders"
    assert config["database_username"] == "orders_user"
    assert config["database_password"] == "orders_pw"
    assert config["admin_password"] == "admin_pw"
    assert config["container_name"] == "oracle-box"
    assert config["startup_sql_scripts"] == ["init.sql"]
