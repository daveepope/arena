import asyncio
import json
import logging

import pytest

from arena_pytest.closed_arena import ClosedArena, _normalize_optional_id_seq


@pytest.mark.parametrize("seq", [None, [], ()])
def test_normalize_optional_id_seq_empty_input_returns_empty_tuple(seq):
    assert _normalize_optional_id_seq(seq) == ()


def test_normalize_optional_id_seq_identifiers_are_stripped_and_kept_in_order():
    assert _normalize_optional_id_seq(["  postgres ", "api"]) == ("postgres", "api")


def test_normalize_optional_id_seq_blank_identifiers_are_dropped():
    assert _normalize_optional_id_seq(["postgres", "   ", ""]) == ("postgres",)


def test_normalize_optional_id_seq_non_string_identifiers_are_coerced():
    assert _normalize_optional_id_seq([1, 2]) == ("1", "2")


def _open_and_close(closed: ClosedArena) -> None:
    async def run() -> None:
        opened = await closed.open()
        await opened.close()

    asyncio.run(run())


def test_open_logger_configured_opens_and_closes_cleanly():
    closed = ClosedArena(
        "py-closed-arena-logger",
        [],
        logger=logging.getLogger("arena-pytest-closed-arena-logger"),
    )

    _open_and_close(closed)


def test_open_logger_factory_configured_asks_the_factory_for_a_logger():
    requested = []

    def factory(name: str) -> logging.Logger:
        requested.append(name)
        return logging.getLogger(f"arena-pytest-closed-arena-factory.{name}")

    closed = ClosedArena("py-closed-arena-factory", [], logger_factory=factory)

    _open_and_close(closed)

    assert requested


class _StubMatch:
    def _for_ffi(self):
        return {"match_name": "m", "dependencies": [], "components": []}


def test_config_no_cpu_profiler_omits_the_cpu_profiled_components_key():
    closed = ClosedArena("arena", [_StubMatch()])

    assert "cpu_profiled_components" not in json.loads(closed._config())


def test_with_cpu_profiler_single_component_serializes_that_id():
    closed = (
        ClosedArena("arena", [_StubMatch()])
        .with_lifecycle_dashboard("http://127.0.0.1:65535")
        .with_cpu_profiler("svc")
        .done()
    )

    assert json.loads(closed._config())["cpu_profiled_components"] == ["svc"]


def test_with_cpu_profiler_several_components_preserve_declaration_order():
    closed = (
        ClosedArena("arena", [_StubMatch()])
        .with_lifecycle_dashboard("http://127.0.0.1:65535")
        .with_cpu_profiler("svc")
        .with_cpu_profiler("worker")
        .done()
    )

    assert json.loads(closed._config())["cpu_profiled_components"] == ["svc", "worker"]


def test_done_after_with_cpu_profiler_returns_the_originating_closed_arena():
    closed = ClosedArena("arena", [_StubMatch()])

    assert closed.with_lifecycle_dashboard().with_cpu_profiler("svc").done() is closed


def test_with_cpu_profile_limit_bytes_set_serializes_the_limit():
    closed = (
        ClosedArena("arena", [_StubMatch()])
        .with_lifecycle_dashboard("http://127.0.0.1:65535")
        .with_cpu_profiler("svc")
        .with_cpu_profile_limit_bytes(1024)
        .done()
    )

    assert json.loads(closed._config())["cpu_profile_limit_bytes"] == 1024


def test_config_no_cpu_profile_limit_omits_the_cpu_profile_limit_key():
    closed = (
        ClosedArena("arena", [_StubMatch()])
        .with_lifecycle_dashboard("http://127.0.0.1:65535")
        .with_cpu_profiler("svc")
        .done()
    )

    assert "cpu_profile_limit_bytes" not in json.loads(closed._config())
