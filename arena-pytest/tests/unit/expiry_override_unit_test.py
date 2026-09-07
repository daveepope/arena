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
    pytest.param(lambda: HttpDependencyBuilder("dep"), id="http"),
    pytest.param(lambda: KafkaDependencyBuilder("dep"), id="kafka"),
    pytest.param(lambda: LocalstackDependencyBuilder("dep"), id="localstack"),
    pytest.param(lambda: MssqlDependencyBuilder("dep"), id="mssql"),
    pytest.param(lambda: OracleDependencyBuilder("dep"), id="oracle"),
    pytest.param(lambda: PostgresDependencyBuilder("dep"), id="postgres"),
    pytest.param(lambda: SmtpDependencyBuilder("dep"), id="smtp"),
    pytest.param(lambda: TemporalDependencyBuilder("dep"), id="temporal"),
    pytest.param(
        lambda: ContainerizedComponentBuilder("web", "Containerfile"),
        id="containerized-component",
    ),
]


def _payload(builder):
    return builder.build()._for_ffi()


@pytest.mark.parametrize("new_builder", BUILDERS)
def test_build_without_expiry_override_omits_expiry_seconds(new_builder):
    assert "expiry_seconds" not in _payload(new_builder())


@pytest.mark.parametrize("new_builder", BUILDERS)
def test_with_expiry_thirty_seconds_sets_expiry_seconds(new_builder):
    payload = _payload(new_builder().with_expiry(timedelta(seconds=30)))

    assert payload["expiry_seconds"] == 30


@pytest.mark.parametrize("new_builder", BUILDERS)
def test_without_expiry_called_sets_expiry_seconds_to_zero(new_builder):
    assert _payload(new_builder().without_expiry())["expiry_seconds"] == 0


@pytest.mark.parametrize("new_builder", BUILDERS)
def test_with_expiry_sub_second_clamps_to_one_second(new_builder):
    payload = _payload(new_builder().with_expiry(timedelta(milliseconds=500)))

    assert payload["expiry_seconds"] == 1


@pytest.mark.parametrize("new_builder", BUILDERS)
def test_with_expiry_negative_raises_value_error(new_builder):
    with pytest.raises(ValueError):
        new_builder().with_expiry(timedelta(milliseconds=-500))


@pytest.mark.parametrize("new_builder", BUILDERS)
def test_with_expiry_zero_sets_expiry_seconds_to_zero(new_builder):
    payload = _payload(new_builder().with_expiry(timedelta(0)))

    assert payload["expiry_seconds"] == 0
