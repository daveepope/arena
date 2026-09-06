from datetime import timedelta

import pytest

from arena_pytest.dep.http import HttpDependencyBuilder
from arena_pytest.dep.kafka import KafkaDependencyBuilder
from arena_pytest.dep.localstack import LocalstackDependencyBuilder
from arena_pytest.dep.mssql import MssqlDependencyBuilder
from arena_pytest.dep.oracle import OracleDependencyBuilder
from arena_pytest.dep.postgres import PostgresDependencyBuilder
from arena_pytest.dep.smtp import SmtpDependencyBuilder
from arena_pytest.dep.temporal import TemporalDependencyBuilder
from arena_pytest.exec.containerized_component import ContainerizedComponentBuilder

BUILDERS = [
    HttpDependencyBuilder,
    KafkaDependencyBuilder,
    LocalstackDependencyBuilder,
    MssqlDependencyBuilder,
    OracleDependencyBuilder,
    PostgresDependencyBuilder,
    SmtpDependencyBuilder,
    TemporalDependencyBuilder,
]


def _config(builder):
    return builder._config


@pytest.mark.parametrize("builder_type", BUILDERS)
def test_build_without_expiry_override_omits_expiry_seconds(builder_type):
    assert "expiry_seconds" not in _config(builder_type("dep"))


@pytest.mark.parametrize("builder_type", BUILDERS)
def test_with_expiry_thirty_seconds_sets_expiry_seconds(builder_type):
    assert _config(builder_type("dep").with_expiry(timedelta(seconds=30)))["expiry_seconds"] == 30


@pytest.mark.parametrize("builder_type", BUILDERS)
def test_without_expiry_called_sets_expiry_seconds_to_zero(builder_type):
    assert _config(builder_type("dep").without_expiry())["expiry_seconds"] == 0


def test_containerized_component_with_expiry_sets_expiry_seconds():
    builder = ContainerizedComponentBuilder("web", "Containerfile").with_expiry(timedelta(seconds=45))

    assert builder._config["expiry_seconds"] == 45


def test_containerized_component_without_expiry_sets_expiry_seconds_to_zero():
    builder = ContainerizedComponentBuilder("web", "Containerfile").without_expiry()

    assert builder._config["expiry_seconds"] == 0


@pytest.mark.parametrize("builder_type", BUILDERS)
def test_with_expiry_sub_second_clamps_to_one_second(builder_type):
    config = _config(builder_type("dep").with_expiry(timedelta(milliseconds=500)))

    assert config["expiry_seconds"] == 1


@pytest.mark.parametrize("builder_type", BUILDERS)
def test_with_expiry_negative_raises_value_error(builder_type):
    with pytest.raises(ValueError):
        builder_type("dep").with_expiry(timedelta(milliseconds=-500))


@pytest.mark.parametrize("builder_type", BUILDERS)
def test_with_expiry_zero_sets_expiry_seconds_to_zero(builder_type):
    config = _config(builder_type("dep").with_expiry(timedelta(0)))

    assert config["expiry_seconds"] == 0
