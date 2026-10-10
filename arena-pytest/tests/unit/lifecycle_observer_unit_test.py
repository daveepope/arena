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

    assert "closing summary | state=arena_closed | faults=0" in lines


def test_arena_state_document_closed_handle_raises_binding_error(arena_ffi):
    from arena_pytest.ffi._ffi import ArenaBindingError, arena_state_document

    with pytest.raises(ArenaBindingError, match="closed arena"):
        arena_state_document(arena_ffi, 0)


def test_observe_single_callback_invoked_on_matching_state(arena_ffi):
    documents: list[str] = []
    closed = ClosedArena("py-observe-single", []).observe(documents.append)

    async def run():
        opened = await closed.open()
        await opened.close()

    asyncio.run(run())

    assert documents, "no transitions were observed"
    assert all(json.loads(d)["id"] == "py-observe-single" for d in documents)


def test_observe_multiple_callbacks_all_invoked(arena_ffi):
    first: list[str] = []
    second: list[str] = []
    closed = (
        ClosedArena("py-observe-multiple", [])
        .observe(first.append)
        .observe(second.append)
    )

    async def run():
        opened = await closed.open()
        await opened.close()

    asyncio.run(run())

    assert first
    assert second
    assert _states_of(first) == _states_of(second)


def test_observe_different_arena_id_not_invoked(arena_ffi):
    other_arena_documents: list[str] = []
    closed_other = ClosedArena("py-observe-other", []).observe(
        other_arena_documents.append
    )

    async def run():
        opened = await closed_other.open()
        await opened.close()
        _open_and_close_plain_arena(arena_ffi, b"py-observe-unrelated")

    asyncio.run(run())

    assert other_arena_documents
    assert all(
        json.loads(d)["id"] == "py-observe-other" for d in other_arena_documents
    )


def test_open_registrationfailure_unregisters_prior_observers(arena_ffi, monkeypatch):
    import arena_pytest.closed_arena as closed_arena_module
    from arena_pytest.ffi._ffi import ArenaBindingError

    real_register = closed_arena_module.register_lifecycle_observer
    real_unregister = closed_arena_module.unregister_lifecycle_observer
    registrations = 0
    issued_tokens: list[int] = []
    unregistered_tokens: list[int] = []

    def flaky_register(ffi, callback):
        nonlocal registrations
        registrations += 1
        if registrations == 2:
            raise ArenaBindingError("registration refused")
        token = real_register(ffi, callback)
        issued_tokens.append(token)
        return token

    def tracking_unregister(ffi, token):
        unregistered_tokens.append(token)
        real_unregister(ffi, token)

    monkeypatch.setattr(closed_arena_module, "register_lifecycle_observer", flaky_register)
    monkeypatch.setattr(closed_arena_module, "unregister_lifecycle_observer", tracking_unregister)

    closed = (
        ClosedArena("py-observe-registration-failure", [])
        .observe(lambda _: None)
        .observe(lambda _: None)
    )

    with pytest.raises(ArenaBindingError):
        asyncio.run(closed.open())

    assert issued_tokens
    assert unregistered_tokens == issued_tokens


def test_close_after_observe_unregisters_cleanly(arena_ffi):
    documents: list[str] = []
    closed = ClosedArena("py-observe-teardown", []).observe(documents.append)

    async def run():
        opened = await closed.open()
        await opened.close()

    asyncio.run(run())
    count_after_first_close = len(documents)

    _open_and_close_plain_arena(arena_ffi, b"py-observe-teardown-unrelated")

    assert len(documents) == count_after_first_close
