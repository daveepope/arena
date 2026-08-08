from unittest.mock import MagicMock

import pytest

import sys
import types

from arena_pytest.arena import pytest_runtest_setup, pytest_runtest_teardown
from arena_pytest.playbook import ActivePlaybook, ManagedPlaybook, UnmanagedPlaybook, playbook


def _arena_plugin():
    mod = sys.modules.get("arena_pytest.arena")
    if isinstance(mod, types.ModuleType):
        return mod
    import arena_pytest.arena as arena_mod

    return (
        arena_mod
        if isinstance(arena_mod, types.ModuleType)
        else sys.modules["arena_pytest.arena"]
    )


class _HookUnmanagedPlaybook(UnmanagedPlaybook):
    def __init__(self, label: str):
        self._label = label

    def identifier(self):
        return self._label

    def run(self, arena):
        return ActivePlaybook(arena._ffi, id(self))


class _HookManagedPlaybook(ManagedPlaybook):
    def __init__(self, label: str):
        self._label = label

    def identifier(self):
        return self._label

    def run(self, arena):
        return ActivePlaybook(arena._ffi, id(self))


class _RecordingClosedArena:
    def __init__(self, matches):
        self._matches = matches


class _FakeOpenArena:
    def __init__(self):
        self._ffi = object()


class _Item:
    def __init__(self, marks):
        self.own_markers = marks
        self._request = None


def test_pytest_runtest_setup_without_marker_is_noop():
    item = _Item([])
    pytest_runtest_setup(item)


def test_pytest_runtest_teardown_without_actives_is_noop():
    item = _Item([])
    pytest_runtest_teardown(item, None)


def test_pytest_runtest_setup_teardown_activates_stacked_unmanaged_markers(monkeypatch):
    opened = []
    dropped = []

    def fake_activate(_arena, _matches, classes):
        opened.extend(k.__name__ for k in classes)
        return [ActivePlaybook(object(), i) for i in range(len(classes))]

    def fake_drop(actives):
        dropped.append(len(actives))

    monkeypatch.setattr(_arena_plugin(), "_activate_classes", fake_activate)
    monkeypatch.setattr(_arena_plugin(), "_drop_actives", fake_drop)

    class Alpha(_HookUnmanagedPlaybook):
        def __init__(self):
            super().__init__("alpha")

    class Beta(_HookUnmanagedPlaybook):
        def __init__(self):
            super().__init__("beta")

    def fn():
        pass

    fn.pytestmark = [playbook(Alpha).mark, playbook(Beta).mark]
    item = _Item(list(fn.pytestmark))
    request = MagicMock()
    request.getfixturevalue = lambda name: (
        _FakeOpenArena() if name == "arena" else _RecordingClosedArena([])
    )
    item._request = request

    pytest_runtest_setup(item)
    assert opened == ["Alpha", "Beta"]

    pytest_runtest_teardown(item, None)
    assert dropped == [2]


def test_pytest_runtest_setup_teardown_defers_managed_marker_to_teardown(monkeypatch):
    run_managed = []

    def fake_run_managed(_arena, _matches, classes):
        run_managed.extend(k.__name__ for k in classes)

    monkeypatch.setattr(_arena_plugin(), "_run_managed_classes", fake_run_managed)

    class Cleanup(_HookManagedPlaybook):
        def __init__(self):
            super().__init__("cleanup")

    def fn():
        pass

    fn.pytestmark = [playbook(Cleanup).mark]
    item = _Item(list(fn.pytestmark))
    request = MagicMock()
    request.getfixturevalue = lambda name: (
        _FakeOpenArena() if name == "arena" else _RecordingClosedArena([])
    )
    item._request = request

    pytest_runtest_setup(item)
    assert run_managed == []

    pytest_runtest_teardown(item, None)
    assert run_managed == ["Cleanup"]
