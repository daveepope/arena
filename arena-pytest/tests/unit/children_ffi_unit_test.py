import pytest

from arena_pytest.dep.http import HttpDependencyBuilder
from arena_pytest.dep.kafka import KafkaDependencyBuilder
from arena_pytest.dep.localstack import LocalstackDependencyBuilder
from arena_pytest.dep.mssql import MssqlDependencyBuilder
from arena_pytest.dep.postgres import PostgresDependencyBuilder
from arena_pytest.dep.smtp import SmtpDependencyBuilder
from arena_pytest.dep.temporal import TemporalDependencyBuilder
from arena_pytest.exec.containerized_component import ContainerizedComponentBuilder
from arena_pytest.exec.executable_component import ExecutableComponentBuilder
from arena_pytest.oauth import OauthDependencyBuilder

_REMAINING_DEPENDENCY_BUILDER_FACTORIES = [
    (lambda name: KafkaDependencyBuilder(name), "kafka"),
    (lambda name: LocalstackDependencyBuilder(name), "localstack"),
    (lambda name: MssqlDependencyBuilder(name), "mssql"),
    (lambda name: OauthDependencyBuilder(name), "oauth"),
    (lambda name: PostgresDependencyBuilder(name), "postgres"),
    (lambda name: SmtpDependencyBuilder(name), "smtp"),
    (lambda name: TemporalDependencyBuilder(name), "temporal"),
]


def test_http_dependency_with_no_children_omits_children_key():
    config = HttpDependencyBuilder("parent").build()._for_ffi()
    assert "children" not in config


def test_http_dependency_with_child_dependencies_nests_child_config():
    child = HttpDependencyBuilder("child").with_port(9090).build()
    config = (
        HttpDependencyBuilder("parent")
        .with_child_dependencies([child])
        .build()
        ._for_ffi()
    )
    assert len(config["children"]) == 1
    assert config["children"][0]["type"] == "http"
    assert config["children"][0]["port"] == 9090
    assert config["children"][0]["identifier"] == child.identifier


def test_executable_component_with_no_children_omits_children_key():
    config = (
        ExecutableComponentBuilder("parent")
        .with_executable_path("/bin/true")
        .build()
        ._for_ffi()
    )
    assert "children" not in config


def test_executable_component_with_child_components_nests_child_config():
    child = ExecutableComponentBuilder("child").with_executable_path("/bin/true").build()
    config = (
        ExecutableComponentBuilder("parent")
        .with_executable_path("/bin/true")
        .with_child_components([child])
        .build()
        ._for_ffi()
    )
    assert len(config["children"]) == 1
    assert config["children"][0]["type"] == "exec"
    assert config["children"][0]["executable_path"] == "/bin/true"


@pytest.mark.parametrize(
    "builder_factory,expected_type", _REMAINING_DEPENDENCY_BUILDER_FACTORIES
)
def test_dependency_with_no_children_omits_children_key(builder_factory, expected_type):
    config = builder_factory("parent").build()._for_ffi()
    assert "children" not in config


@pytest.mark.parametrize(
    "builder_factory,expected_type", _REMAINING_DEPENDENCY_BUILDER_FACTORIES
)
def test_dependency_with_child_dependencies_nests_child_config(builder_factory, expected_type):
    child = builder_factory("child").build()
    config = (
        builder_factory("parent")
        .with_child_dependencies([child])
        .build()
        ._for_ffi()
    )
    assert len(config["children"]) == 1
    assert config["children"][0]["type"] == expected_type
    assert config["children"][0]["identifier"] == child.identifier


def test_containerized_component_with_no_children_omits_children_key():
    config = (
        ContainerizedComponentBuilder("parent", "Dockerfile")
        .build()
        ._for_ffi()
    )
    assert "children" not in config


def test_containerized_component_with_child_components_nests_child_config():
    child = ContainerizedComponentBuilder("child", "Dockerfile").build()
    child_config = child._for_ffi()
    config = (
        ContainerizedComponentBuilder("parent", "Dockerfile")
        .with_child_components([child])
        .build()
        ._for_ffi()
    )
    assert len(config["children"]) == 1
    assert config["children"][0]["type"] == "container"
    assert config["children"][0]["identifier"] == child_config["identifier"]
