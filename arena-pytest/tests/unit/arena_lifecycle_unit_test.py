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
        lambda ffi, handle, dispatcher_logging_target_token=0: None,
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
