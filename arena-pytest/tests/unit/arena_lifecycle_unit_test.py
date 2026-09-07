import asyncio
import sys
import types
from unittest.mock import MagicMock

import pytest

from arena_pytest.arena import (
    OpenArena,
    _item_request,
    active_playbooks_for_item,
    pytest_configure,
)


def _arena_module():
    mod = sys.modules.get("arena_pytest.arena")
    if isinstance(mod, types.ModuleType):
        return mod
    import arena_pytest.arena as arena_mod

    return arena_mod if isinstance(arena_mod, types.ModuleType) else sys.modules["arena_pytest.arena"]


def test_pytest_configure_registers_playbook_marker_and_asyncio_defaults():
    config = MagicMock()
    config.getini = lambda key: {
        "asyncio_mode": None,
        "asyncio_default_fixture_loop_scope": "",
    }.get(key)
    config._inicache = {}

    pytest_configure(config)

    config.addinivalue_line.assert_called_once()
    assert config._inicache["asyncio_mode"] == "auto"
    assert config._inicache["asyncio_default_fixture_loop_scope"] == "session"


def test_pytest_configure_preserves_existing_asyncio_settings():
    config = MagicMock()
    config.getini = lambda key: {
        "asyncio_mode": "auto",
        "asyncio_default_fixture_loop_scope": "session",
    }.get(key)
    config._inicache = {}

    pytest_configure(config)

    assert config._inicache == {}


def test_item_request_without_request_raises_usage_error():
    item = MagicMock(spec=[])
    with pytest.raises(pytest.UsageError, match="arena' fixture available"):
        _item_request(item)


def test_item_request_returns_attached_request():
    item = MagicMock()
    item._request = sentinel = object()
    assert _item_request(item) is sentinel


def test_open_arena_close_clears_handle(monkeypatch):
    arena_mod = _arena_module()
    closed = []

    async def fake_to_thread(fn, *args, **kwargs):
        closed.append((fn, args, kwargs))
        fn(*args, **kwargs)

    monkeypatch.setattr(asyncio, "to_thread", fake_to_thread)
    monkeypatch.setattr(
        arena_mod,
        "close_arena",
        lambda ffi, handle, dispatcher_logging_target_token=0, on_state_document=None: None,
    )

    async def run():
        arena = OpenArena(object(), 42, dispatcher_logging_target_token=7)
        await arena.close()
        assert arena.handle() == 0

    asyncio.run(run())
    assert closed


def test_open_arena_reset_delegates_to_ffi(monkeypatch):
    arena_mod = _arena_module()
    calls = []

    async def fake_to_thread(fn, *args, **kwargs):
        return fn(*args, **kwargs)

    monkeypatch.setattr(asyncio, "to_thread", fake_to_thread)
    monkeypatch.setattr(
        arena_mod,
        "ffi_soft_reset",
        lambda ffi, handle, dep: calls.append(("soft", dep)),
    )
    monkeypatch.setattr(
        arena_mod,
        "ffi_hard_reset",
        lambda ffi, handle, dep: calls.append(("hard", dep)),
    )

    async def run():
        arena = OpenArena(object(), 9)
        await arena.soft_reset("dep-a")
        await arena.hard_reset("dep-b")

    asyncio.run(run())

    assert ("soft", "dep-a") in calls
    assert ("hard", "dep-b") in calls


def test_active_playbooks_for_item_empty_when_attr_missing():
    item = MagicMock(spec=[])
    assert active_playbooks_for_item(item) == []


def test_playbook_class_scope_with_markers_runs_activate_and_drop(monkeypatch):
    import inspect

    from arena_pytest.playbook import ActivePlaybook, Playbook, playbook

    arena_mod = _arena_module()
    dropped = []

    class Alpha(Playbook):
        def identifier(self):
            return "alpha"

        def run(self, arena):
            return ActivePlaybook(arena._ffi, 1)

    monkeypatch.setattr(arena_mod, "_class_marker_classes", lambda cls: [Alpha])
    monkeypatch.setattr(arena_mod, "_matches_from_closed_arena", lambda closed: ["match"])
    monkeypatch.setattr(
        arena_mod,
        "_activate_classes",
        lambda arena, matches, classes: [ActivePlaybook(object(), 1)],
    )
    monkeypatch.setattr(
        arena_mod,
        "_finish_playbook_scope",
        lambda arena, matches, actives, managed: dropped.append(len(actives)),
    )

    request = MagicMock()
    request.cls = object()
    request.getfixturevalue = lambda name: (
        OpenArena(object(), 3) if name == "arena" else object()
    )

    scope_fn = inspect.unwrap(arena_mod._playbook_class_scope)
    gen = scope_fn(request)
    next(gen)
    with pytest.raises(StopIteration):
        gen.send(None)

    assert dropped == [1]


def test_playbook_module_scope_with_markers_runs_activate_and_drop(monkeypatch):
    import inspect

    from arena_pytest.playbook import ActivePlaybook, Playbook

    arena_mod = _arena_module()
    dropped = []

    class Beta(Playbook):
        def identifier(self):
            return "beta"

        def run(self, arena):
            return ActivePlaybook(arena._ffi, 2)

    monkeypatch.setattr(arena_mod, "_module_marker_classes", lambda module: [Beta])
    monkeypatch.setattr(arena_mod, "_matches_from_closed_arena", lambda closed: ["match"])
    monkeypatch.setattr(
        arena_mod,
        "_activate_classes",
        lambda arena, matches, classes: [ActivePlaybook(object(), 1)],
    )
    monkeypatch.setattr(
        arena_mod,
        "_finish_playbook_scope",
        lambda arena, matches, actives, managed: dropped.append(len(actives)),
    )

    request = MagicMock()
    request.module = object()
    request.getfixturevalue = lambda name: (
        OpenArena(object(), 4) if name == "arena" else object()
    )

    scope_fn = inspect.unwrap(arena_mod._playbook_module_scope)
    gen = scope_fn(request)
    next(gen)
    with pytest.raises(StopIteration):
        gen.send(None)

    assert dropped == [1]


def test_arena_fixture_skips_when_closed_arena_missing():
    import inspect

    from _pytest.outcomes import Skipped

    arena_mod = _arena_module()
    arena_fn = inspect.unwrap(arena_mod.arena)

    async def run():
        gen = arena_fn(None)
        with pytest.raises(Skipped):
            await gen.__anext__()

    asyncio.run(run())


def test_arena_ffi_fixture_skips_when_library_missing(monkeypatch):
    import inspect

    from _pytest.outcomes import Skipped

    arena_mod = _arena_module()
    monkeypatch.setattr(arena_mod, "load_ffi", lambda: None)
    ffi_fn = inspect.unwrap(arena_mod.arena_ffi)

    with pytest.raises(Skipped):
        ffi_fn()


pytest_plugins = ["pytester"]


def test_arena_fixture_faulted_open_errors_with_rendered_state(pytester):
    pytester.makeconftest(
        """
import pytest

from arena_pytest.closed_arena import ClosedArena


class _BrokenExecMatch:
    def _for_ffi(self):
        return {
            "components": [
                {
                    "type": "exec",
                    "identifier": "fixture-missing-binary",
                    "executable_path": "/nonexistent/fixture-probe",
                }
            ]
        }


@pytest.fixture(scope="session")
def closed_arena():
    return ClosedArena("fixture-faulted-arena", [_BrokenExecMatch()])
"""
    )
    pytester.makepyfile(
        """
def test_never_runs(arena):
    raise AssertionError("the test body must not run when the arena faults")
"""
    )

    result = pytester.runpytest_inprocess("-p", "arena_pytest.arena")

    result.assert_outcomes(errors=1)
    output = result.stdout.str() + result.stderr.str()
    assert "ArenaLifecycleError" in output
    assert "is arena_faulted" in output
    assert "fixture-missing-binary" in output
    assert "panicked at" not in output


def test_pytest_configure_registers_transition_logging(monkeypatch):
    arena_mod = _arena_module()
    monkeypatch.setattr(arena_mod, "_install_sigterm_teardown", lambda: None)
    monkeypatch.setattr(arena_mod, "_lifecycle_observer_token", 0)
    monkeypatch.setattr(arena_mod, "_lifecycle_observer_sessions", 0)
    registered = []
    monkeypatch.setattr(arena_mod, "load_ffi", lambda: object())
    monkeypatch.setattr(
        arena_mod,
        "register_lifecycle_observer",
        lambda ffi, on_state: registered.append(on_state) or 7,
    )
    config = MagicMock()
    config.getini = lambda key: {
        "asyncio_mode": "auto",
        "asyncio_default_fixture_loop_scope": "session",
    }.get(key)
    config._inicache = {}

    arena_mod.pytest_configure(config)

    assert registered == [arena_mod.log_transition_document]
    assert arena_mod._lifecycle_observer_token == 7


def test_pytest_unconfigure_last_session_unregisters_transition_logging(monkeypatch):
    arena_mod = _arena_module()
    monkeypatch.setattr(arena_mod, "_restore_sigterm_handler", lambda: None)
    monkeypatch.setattr(arena_mod, "_lifecycle_observer_token", 7)
    monkeypatch.setattr(arena_mod, "_lifecycle_observer_sessions", 1)
    removed = []
    monkeypatch.setattr(arena_mod, "load_ffi", lambda: object())
    monkeypatch.setattr(
        arena_mod,
        "unregister_lifecycle_observer",
        lambda ffi, token: removed.append(token),
    )

    arena_mod.pytest_unconfigure(MagicMock())

    assert removed == [7]
    assert arena_mod._lifecycle_observer_token == 0


def test_pytest_unconfigure_nested_session_keeps_the_observer(monkeypatch):
    arena_mod = _arena_module()
    monkeypatch.setattr(arena_mod, "_restore_sigterm_handler", lambda: None)
    monkeypatch.setattr(arena_mod, "_lifecycle_observer_token", 7)
    monkeypatch.setattr(arena_mod, "_lifecycle_observer_sessions", 2)
    removed = []
    monkeypatch.setattr(arena_mod, "load_ffi", lambda: object())
    monkeypatch.setattr(
        arena_mod,
        "unregister_lifecycle_observer",
        lambda ffi, token: removed.append(token),
    )

    arena_mod.pytest_unconfigure(MagicMock())

    assert removed == []
    assert arena_mod._lifecycle_observer_token == 7


def test_pytest_configure_missing_lib_leaves_token_unset(monkeypatch):
    arena_mod = _arena_module()
    monkeypatch.setattr(arena_mod, "_install_sigterm_teardown", lambda: None)
    monkeypatch.setattr(arena_mod, "_lifecycle_observer_token", 0)
    monkeypatch.setattr(arena_mod, "_lifecycle_observer_sessions", 0)
    monkeypatch.setattr(arena_mod, "load_ffi", lambda: None)
    config = MagicMock()
    config.getini = lambda key: {
        "asyncio_mode": "auto",
        "asyncio_default_fixture_loop_scope": "session",
    }.get(key)
    config._inicache = {}

    arena_mod.pytest_configure(config)

    assert arena_mod._lifecycle_observer_token == 0
    assert arena_mod._lifecycle_observer_sessions == 1


def test_open_arena_close_called_twice_closes_once(monkeypatch):
    arena_mod = _arena_module()
    calls = []

    async def fake_to_thread(fn, *args, **kwargs):
        return fn(*args, **kwargs)

    monkeypatch.setattr(asyncio, "to_thread", fake_to_thread)
    monkeypatch.setattr(
        arena_mod,
        "close_arena",
        lambda ffi, handle, dispatcher_logging_target_token=0, on_state_document=None: calls.append(
            handle
        ),
    )

    async def run():
        arena = OpenArena(object(), 42)
        await arena.close()
        await arena.close()

    asyncio.run(run())
    assert calls == [42]


def test_open_arena_close_faulted_raises_lifecycle_error_with_state(monkeypatch):
    import json

    from arena_pytest.ffi._ffi import ArenaBindingError
    from arena_pytest.lifecycle import ArenaLifecycleError

    arena_mod = _arena_module()

    async def fake_to_thread(fn, *args, **kwargs):
        return fn(*args, **kwargs)

    def failing_close(ffi, handle, dispatcher_logging_target_token=0, on_state_document=None):
        error = ArenaBindingError("arena close faulted")
        error.state_document = json.dumps(
            {"id": "close-faulted", "state": "arena_faulted", "at": "t"}
        )
        raise error

    monkeypatch.setattr(asyncio, "to_thread", fake_to_thread)
    monkeypatch.setattr(arena_mod, "close_arena", failing_close)

    async def run():
        arena = OpenArena(object(), 42)
        with pytest.raises(ArenaLifecycleError) as raised:
            await arena.close()
        assert raised.value.state.id == "close-faulted"
        assert raised.value.state.is_faulted()
        assert arena.handle() == 0

    asyncio.run(run())
