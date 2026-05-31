import socket
import sys
import os
from unittest.mock import MagicMock

import pytest

from readings_ephemeral_test_runtime import RUNTIME, ReadingsEphemeralTestRuntime, ephemeral_tcp_port


def _playbook_pkg():
    from arena_pytest.playbook import ActivePlaybook as _  # noqa: F401

    return sys.modules["arena_pytest.playbook"]


def test_ephemeral_tcp_port_bind_returns_nonzero_port():
    port = ephemeral_tcp_port()
    assert 1024 <= port <= 65535


def test_readings_ephemeral_runtime_ports_are_pairwise_distinct():
    rt = ReadingsEphemeralTestRuntime()
    ports = (
        rt.exec_web_app_port,
        rt.docker_web_host_port,
        rt.kafka_port,
        rt.calibration_host_port,
        rt.postgres_port,
        rt.mssql_port,
        rt.oauth_port,
        rt.localstack_host_port,
    )
    assert len(ports) == len(set(ports))


def test_readings_ephemeral_runtime_suffixes_network_and_container_names():
    assert RUNTIME.network_name("arena-readings-api-network").startswith(
        "arena-readings-api-network-"
    )
    assert RUNTIME.container_name("readings-api-postgres").startswith(
        "readings-api-postgres-"
    )


def test_playbook_marker_two_args_raises_usage_error():
    from arena_pytest.playbook import _resolve_playbook_classes_from_marker

    class _TwoArgMark:
        args = ("A", "B")

    with pytest.raises(pytest.UsageError, match="exactly one Playbook class"):
        _resolve_playbook_classes_from_marker(_TwoArgMark())


def test_playbook_marker_stacked_marks_collect_both_classes():
    from arena_pytest.playbook import Playbook, _own_marker_classes, playbook

    class Alpha(Playbook):
        def identifier(self):
            return "alpha"

        def run(self, arena):
            raise NotImplementedError

    class Beta(Playbook):
        def identifier(self):
            return "beta"

        def run(self, arena):
            raise NotImplementedError

    def sample():
        pass

    sample.pytestmark = [playbook(Alpha).mark, playbook(Beta).mark]
    item = type("_Item", (), {"own_markers": list(sample.pytestmark)})()
    classes = _own_marker_classes(item)
    assert classes == [Alpha, Beta]


def test_active_playbook_drop_after_body_failure_swallows_binding_error(monkeypatch):
    from arena_pytest.ffi._ffi import ArenaBindingError
    from arena_pytest.playbook import ActivePlaybook

    monkeypatch.setattr(
        _playbook_pkg(),
        "active_playbook_drop",
        lambda _ffi, _h: (_ for _ in ()).throw(ArenaBindingError("drop expect failed")),
    )
    active = ActivePlaybook(MagicMock(), 99)
    active._note_body_failure()
    active.__exit__(None, None, None)


def test_active_playbook_drop_without_body_failure_reraises_binding_error(monkeypatch):
    from arena_pytest.ffi._ffi import ArenaBindingError
    from arena_pytest.playbook import ActivePlaybook

    monkeypatch.setattr(
        _playbook_pkg(),
        "active_playbook_drop",
        lambda _ffi, _h: (_ for _ in ()).throw(ArenaBindingError("drop expect failed")),
    )
    active = ActivePlaybook(MagicMock(), 99)
    with pytest.raises(AssertionError, match="drop expect failed"):
        active.__exit__(None, None, None)


if __name__ == "__main__":
    sys.exit(pytest.main([os.path.dirname(os.path.abspath(__file__)), "-v", "-s"]))
