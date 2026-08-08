import sys
import os

import pytest


def _make_managed_playbook(identifier: str = "pb"):
    from arena_pytest.playbook import ActivePlaybook, ManagedPlaybook

    class _Managed(ManagedPlaybook):
        def identifier(self):
            return identifier

        def run(self, arena):
            return ActivePlaybook(None, 0)

        def _for_ffi(self):
            return {"identifier": identifier, "kind": "fake"}

    return _Managed()


def _make_unmanaged_playbook(identifier: str = "pb"):
    from arena_pytest.playbook import ActivePlaybook, UnmanagedPlaybook

    class _Unmanaged(UnmanagedPlaybook):
        def identifier(self):
            return identifier

        def run(self, arena):
            return ActivePlaybook(None, 0)

    return _Unmanaged()


def test_register_playbook_non_ffi_playbook_registers_successfully():
    from arena_pytest.match.matches import MatchBuilder

    playbook = _make_unmanaged_playbook()
    builder = MatchBuilder("m").register_playbook(playbook)
    match = builder.build()
    assert match.playbook(type(playbook)) is playbook


def test_register_playbook_non_ffi_playbook_with_exec_on_dependency_start_raises_type_error():
    from arena_pytest.match.matches import MatchBuilder

    playbook = _make_unmanaged_playbook()
    with pytest.raises(TypeError, match="serializes its manifest"):
        MatchBuilder("m").register_playbook(playbook, exec_on_dependency_start=True)


def test_register_playbook_non_playbook_instance_raises_type_error():
    from arena_pytest.match.matches import MatchBuilder

    with pytest.raises(TypeError, match="ManagedPlaybook or UnmanagedPlaybook"):
        MatchBuilder("m").register_playbook(object())


def test_register_playbook_duplicate_type_raises_value_error():
    from arena_pytest.match.matches import MatchBuilder

    first = _make_unmanaged_playbook("a")
    second = type(first)()
    builder = MatchBuilder("m").register_playbook(first)
    with pytest.raises(ValueError, match="already registered"):
        builder.register_playbook(second)


def test_register_playbook_ffi_managed_playbook_with_exec_on_dependency_start_registers():
    from arena_pytest.match.matches import MatchBuilder

    playbook = _make_managed_playbook()
    builder = MatchBuilder("m").register_playbook(
        playbook, exec_on_dependency_start=True
    )
    match = builder.build()
    assert match.playbook(type(playbook)) is playbook


def test_match_for_ffi_omits_playbooks_without_for_ffi():
    from arena_pytest.match.matches import MatchBuilder

    unmanaged = _make_unmanaged_playbook("no-ffi")
    managed = _make_managed_playbook("has-ffi")
    match = (
        MatchBuilder("m")
        .register_playbook(unmanaged)
        .register_playbook(managed)
        .build()
    )
    payload = match._for_ffi()
    identifiers = [p["identifier"] for p in payload.get("playbooks", [])]
    assert identifiers == ["has-ffi"]


if __name__ == "__main__":
    sys.exit(pytest.main([os.path.dirname(os.path.abspath(__file__)), "-v", "-s"]))
