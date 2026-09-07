import asyncio
import json
import logging

import pytest

from arena_pytest.closed_arena import ClosedArena
from arena_pytest.ffi._ffi import (
    load_ffi,
    open_arena,
    close_arena,
    register_lifecycle_observer,
    unregister_lifecycle_observer,
)
from arena_pytest.lifecycle import ArenaLifecycleError


class _ExecMatch:
    def __init__(self, identifier: str, executable_path: str):
        self._identifier = identifier
        self._executable_path = executable_path

    def _for_ffi(self):
        return {
            "components": [
                {
                    "type": "exec",
                    "identifier": self._identifier,
                    "executable_path": self._executable_path,
                }
            ]
        }


@pytest.fixture
def arena_ffi():
    ffi = load_ffi()
    if ffi is None:
        pytest.skip("arena shared library not found")
    return ffi


def _open_and_close_plain_arena(ffi, name: bytes) -> None:
    handle = open_arena(ffi, name)
    close_arena(ffi, handle)


def _states_of(documents: list[str]) -> list[str]:
    return [json.loads(document)["state"] for document in documents]


def test_register_lifecycle_observer_open_and_close_reports_in_order(arena_ffi):
    documents: list[str] = []
    token = register_lifecycle_observer(arena_ffi, documents.append)
    try:
        _open_and_close_plain_arena(arena_ffi, b"py-observer-order")
    finally:
        unregister_lifecycle_observer(arena_ffi, token)

    states = _states_of(documents)
    assert states, "no transitions were observed"
    assert states[0] == "arena_starting"
    assert states[-1] == "arena_closed"
    assert states.index("arena_open") < states.index("arena_closing")


def test_register_lifecycle_observer_reports_the_arena_identifier(arena_ffi):
    documents: list[str] = []
    token = register_lifecycle_observer(arena_ffi, documents.append)
    try:
        _open_and_close_plain_arena(arena_ffi, b"py-observer-identity")
    finally:
        unregister_lifecycle_observer(arena_ffi, token)

    assert json.loads(documents[0])["id"] == "py-observer-identity"


def test_unregister_lifecycle_observer_stops_further_transitions(arena_ffi):
    documents: list[str] = []
    token = register_lifecycle_observer(arena_ffi, documents.append)
    unregister_lifecycle_observer(arena_ffi, token)

    _open_and_close_plain_arena(arena_ffi, b"py-observer-removed")

    assert documents == []


def test_unregister_lifecycle_observer_zero_token_is_ignored(arena_ffi):
    unregister_lifecycle_observer(arena_ffi, 0)


def test_open_faulted_component_raises_lifecycle_error_with_state(arena_ffi, capfd):
    closed = ClosedArena(
        "py-lifecycle-faulted",
        [_ExecMatch("py-lifecycle-missing-binary", "/nonexistent/py-lifecycle-probe")],
    )

    with pytest.raises(ArenaLifecycleError) as raised:
        asyncio.run(closed.open())

    error = raised.value
    assert "is arena_faulted" in str(error)
    assert error.state is not None
    assert error.state.is_faulted()
    assert error.state.id == "py-lifecycle-faulted"
    component = next(
        (c for c in error.state.components if "py-lifecycle-missing-binary" in c.id),
        None,
    )
    assert component is not None
    captured = capfd.readouterr()
    assert "panicked at" not in captured.out + captured.err


def test_open_arena_state_accessor_returns_open_state(arena_ffi):
    closed = ClosedArena("py-state-accessor", [])

    async def run():
        opened = await closed.open()
        state = await opened.state()
        await opened.close()
        return state

    state = asyncio.run(run())

    assert state.id == "py-state-accessor"
    assert state.state == "arena_open"


def test_open_arena_close_logs_the_closing_summary(arena_ffi):
    closed = ClosedArena("py-close-summary", [])
    lg = logging.getLogger("arena.py-close-summary")
    lines: list[str] = []

    class _Capture(logging.Handler):
        def emit(self, record: logging.LogRecord) -> None:
            lines.append(record.getMessage())

    capture = _Capture()
    previous_level = lg.level
    lg.addHandler(capture)
    lg.setLevel(logging.DEBUG)
    try:
        async def run():
            opened = await closed.open()
            await opened.close()

        asyncio.run(run())
    finally:
        lg.removeHandler(capture)
        lg.setLevel(previous_level)

    assert "closing summary | state=arena_closed, faults=0" in lines


def test_arena_state_document_closed_handle_raises_binding_error(arena_ffi):
    from arena_pytest.ffi._ffi import ArenaBindingError, arena_state_document

    with pytest.raises(ArenaBindingError, match="closed arena"):
        arena_state_document(arena_ffi, 0)
