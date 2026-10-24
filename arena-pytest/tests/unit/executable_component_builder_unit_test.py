import pytest

from arena_pytest.exec.executable_component import (
    BuildTool,
    ExecutableComponentBuilder,
)
from arena_pytest.readiness import HttpReadinessCheck, TcpReadinessCheck


def test_build_minimal_name_serializes_exec_type_with_empty_env_and_args():
    config = ExecutableComponentBuilder("api").build()._for_ffi()

    assert config["type"] == "exec"
    assert config["env_vars"] == {}
    assert config["runtime_args"] == []
    assert config["identifier"].startswith("arena-executable-component-")


def test_identifier_built_component_matches_serialized_identifier():
    component = ExecutableComponentBuilder("api").build()

    assert component.identifier == component._for_ffi()["identifier"]


@pytest.mark.parametrize(
    ("configure", "key", "expected"),
    [
        (lambda b: b.with_executable_path("/bin/true"), "executable_path", "/bin/true"),
        (lambda b: b.with_source_path("/src/api"), "source_path", "/src/api"),
        (lambda b: b.with_build_tool(BuildTool.PYTHON), "build_tool", "python"),
    ],
)
def test_builder_single_option_sets_its_own_config_key(configure, key, expected):
    config = configure(ExecutableComponentBuilder("api")).build()._for_ffi()

    assert config[key] == expected


def test_with_build_tool_custom_command_and_args_serializes_both():
    config = (
        ExecutableComponentBuilder("api")
        .with_build_tool_custom("just", ["build", "--release"])
        .build()
        ._for_ffi()
    )

    assert config["build_tool"] == {"command": "just", "args": ["build", "--release"]}


def test_with_env_var_multiple_keys_serializes_every_pair():
    config = (
        ExecutableComponentBuilder("api")
        .with_env_var("FIRST", "one")
        .with_env_var("SECOND", "two")
        .build()
        ._for_ffi()
    )

    assert config["env_vars"] == {"FIRST": "one", "SECOND": "two"}


def test_with_runtime_arg_multiple_args_preserve_declaration_order():
    config = (
        ExecutableComponentBuilder("api")
        .with_runtime_arg("flag", "-c")
        .with_runtime_arg("script", "print(1)")
        .build()
        ._for_ffi()
    )

    assert config["runtime_args"] == [
        {"name": "flag", "value": "-c"},
        {"name": "script", "value": "print(1)"},
    ]


def test_with_child_components_nested_child_serializes_under_children():
    child = ExecutableComponentBuilder("worker").build()

    config = (
        ExecutableComponentBuilder("api")
        .with_child_components([child])
        .build()
        ._for_ffi()
    )

    assert len(config["children"]) == 1
    assert config["children"][0]["identifier"] == child.identifier


def test_build_no_children_omits_the_children_key():
    config = ExecutableComponentBuilder("api").build()._for_ffi()

    assert "children" not in config


def test_builder_every_option_chained_returns_the_same_builder():
    builder = ExecutableComponentBuilder("api")

    chained = (
        builder.with_executable_path("/bin/true")
        .with_source_path("/src/api")
        .with_build_tool(BuildTool.MAKE)
        .with_env_var("KEY", "value")
        .with_runtime_arg("flag", "-v")
        .with_child_components([])
    )

    assert chained is builder


@pytest.mark.parametrize(
    ("check", "target", "kind"),
    [
        (HttpReadinessCheck.create(), "http://127.0.0.1:8080/health", "http"),
        (TcpReadinessCheck.create(), "127.0.0.1:2525", "tcp"),
    ],
)
def test_with_readiness_check_single_check_serializes_its_kind_and_target(check, target, kind):
    config = (
        ExecutableComponentBuilder("api")
        .with_readiness_check(check, target, 500)
        .build()
        ._for_ffi()
    )

    assert config["readiness_checks"] == [
        {"kind": kind, "target": target, "timeout_ms": 500}
    ]


def test_with_readiness_check_default_timeout_serializes_ten_seconds():
    config = (
        ExecutableComponentBuilder("api")
        .with_readiness_check(HttpReadinessCheck.create(), "http://127.0.0.1:9/health")
        .build()
        ._for_ffi()
    )

    assert config["readiness_checks"][0]["timeout_ms"] == 10_000


def test_builder_for_ffi_no_children_omits_the_children_key():
    config = ExecutableComponentBuilder("api")._for_ffi()

    assert config["type"] == "exec"
    assert "children" not in config


def test_builder_for_ffi_with_children_serializes_them_under_children():
    child = ExecutableComponentBuilder("worker").build()

    config = ExecutableComponentBuilder("api").with_child_components([child])._for_ffi()

    assert len(config["children"]) == 1
    assert config["children"][0]["identifier"] == child.identifier
