import asyncio
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
