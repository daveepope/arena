import uuid

from arena_pytest.dep.oracle import ManagedOraclePlaybook, OracleDependencyBuilder
from arena_pytest.playbook import ActiveOraclePlaybook


def _random_password() -> str:
    return f"pw-{uuid.uuid4()}"


def test_build_minimal_name_serializes_type_and_identifier():
    config = OracleDependencyBuilder("oracle").build()._for_ffi()
    assert config["type"] == "oracle"
    assert config["identifier"].startswith("arena-oracle-oracle-")
    assert "image" not in config
    assert "port" not in config
    assert "admin_password" not in config


def test_with_admin_password_sets_admin_password():
    admin_password = _random_password()
    config = OracleDependencyBuilder("oracle").with_admin_password(admin_password).build()._for_ffi()
    assert config["admin_password"] == admin_password


def test_build_with_overrides_serializes_configured_fields():
    database_password = _random_password()
    admin_password = _random_password()
    config = (
        OracleDependencyBuilder("oracle")
        .with_image("23.5.0.24.07")
        .with_image_name("gvenzl/oracle-free")
        .with_port(11521)
        .with_database_name("orders")
        .with_database_username("orders_user")
        .with_database_password(database_password)
        .with_admin_password(admin_password)
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
    assert config["database_password"] == database_password
    assert config["admin_password"] == admin_password
    assert config["container_name"] == "oracle-box"
    assert config["startup_sql_scripts"] == ["init.sql"]


def test_with_startup_sql_scripts_alone_sets_scripts_list():
    config = (
        OracleDependencyBuilder("oracle")
        .with_startup_sql_scripts(["seed.sql", "grants.sql"])
        .build()
        ._for_ffi()
    )
    assert config["startup_sql_scripts"] == ["seed.sql", "grants.sql"]


def test_build_minimal_name_omits_startup_sql_scripts():
    config = OracleDependencyBuilder("oracle").build()._for_ffi()
    assert "startup_sql_scripts" not in config


def test_with_child_dependencies_nests_child_config_under_parent():
    child = OracleDependencyBuilder("child").with_port(11522).build()
    config = (
        OracleDependencyBuilder("parent")
        .with_child_dependencies([child])
        .build()
        ._for_ffi()
    )
    assert len(config["children"]) == 1
    assert config["children"][0]["type"] == "oracle"
    assert config["children"][0]["port"] == 11522
    assert config["children"][0]["identifier"] == child.identifier


def test_build_with_no_children_omits_children_key():
    config = OracleDependencyBuilder("oracle").build()._for_ffi()
    assert "children" not in config


def test_builder_for_ffi_with_child_dependencies_nests_child_config():
    child = OracleDependencyBuilder("child").build()
    config = OracleDependencyBuilder("parent").with_child_dependencies([child])._for_ffi()
    assert len(config["children"]) == 1
    assert config["children"][0]["identifier"] == child.identifier


def test_builder_for_ffi_with_no_children_omits_children_key():
    config = OracleDependencyBuilder("oracle")._for_ffi()
    assert "children" not in config


def test_dependency_identifier_returns_configured_identifier():
    dependency = OracleDependencyBuilder("oracle").build()
    assert dependency.identifier == dependency._config["identifier"]


def test_managed_oracle_playbook_identifier_returns_configured_identifier():
    pb = ManagedOraclePlaybook(identifier="pb-1", dependency_identifier="dep-1")
    assert pb.identifier() == "pb-1"
    assert pb.dependency_identifier == "dep-1"


def test_managed_oracle_playbook_for_ffi_serializes_kind_and_identifiers():
    pb = ManagedOraclePlaybook(identifier="pb-1", dependency_identifier="dep-1")
    assert pb._for_ffi() == {
        "identifier": "pb-1",
        "kind": "oracle",
        "dependency_identifier": "dep-1",
    }


def test_managed_oracle_playbook_run_returns_active_oracle_playbook(monkeypatch):
    import arena_pytest.dep.oracle as oracle_module

    monkeypatch.setattr(
        oracle_module, "match_playbook_run", lambda ffi, handle, identifier: 42
    )
    pb = ManagedOraclePlaybook(identifier="pb-1", dependency_identifier="dep-1")

    class _FakeOpenArena:
        _ffi = object()
        _handle = 7

    active = pb.run(_FakeOpenArena())

    assert isinstance(active, ActiveOraclePlaybook)
    assert active.handle() == 42
    assert active._dependency_identifier == "dep-1"
