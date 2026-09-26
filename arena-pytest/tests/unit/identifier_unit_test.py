import pytest

from arena_pytest.support._identifier import build

MODULES = [
    "arena-http",
    "arena-kafka",
    "arena-localstack",
    "arena-mssql",
    "arena-oracle",
    "arena-postgres",
    "arena-smtp",
    "arena-temporal",
]


@pytest.mark.parametrize("name", ["oracle", "broker", "server", "kafka1"])
def test_build_six_character_name_appends_suffix(name):
    built = build("arena-postgres", name)

    assert built.startswith(f"arena-postgres-{name}-")
    assert built != name


@pytest.mark.parametrize("module", MODULES)
def test_build_same_name_twice_produces_different_identifiers(module):
    assert build(module, "oracle") != build(module, "oracle")


def test_build_already_built_identifier_is_unchanged():
    once = build("arena-postgres", "orders")

    assert build("arena-postgres", once) == once


def test_build_identifier_built_by_another_module_is_preserved():
    assert build("arena-postgres", "arena-oracle-api-oracle-a1b2c3") == (
        "arena-oracle-api-oracle-a1b2c3"
    )
