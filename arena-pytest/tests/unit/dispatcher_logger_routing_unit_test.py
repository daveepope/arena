import logging

import pytest

from arena_pytest.ffi._ffi import ArenaLogLevel, _UserDispatcherLoggerBridge


class _NativeLibStub:
    lib = object()


def _bridge_for_factory(factory):
    return _UserDispatcherLoggerBridge(
        _NativeLibStub(), None, ArenaLogLevel.INFO, logger_factory=factory
    )


def _recording_factory():
    requested = []

    def factory(name):
        requested.append(name)
        return logging.getLogger(name)

    return factory, requested


@pytest.mark.parametrize(
    "target,expected_name",
    [
        ("arena.orders", "arena.orders"),
        ("arena.orders.dependency.orders-postgres", "arena.orders.dependency.orders-postgres"),
        ("", "arena"),
    ],
)
def test_logger_for_record_target_resolves_through_the_factory(target, expected_name):
    factory, requested = _recording_factory()
    bridge = _bridge_for_factory(factory)

    bridge._logger_for(target)

    assert requested == [expected_name]


def test_logger_for_repeated_target_reuses_the_cached_logger():
    factory, requested = _recording_factory()
    bridge = _bridge_for_factory(factory)

    first = bridge._logger_for("arena.orders")
    second = bridge._logger_for("arena.orders")

    assert first is second
    assert requested == ["arena.orders"]


def test_logger_for_bare_logger_returns_that_logger():
    lg = logging.getLogger("app.under.test.bare-logger-routing")
    bridge = _UserDispatcherLoggerBridge(_NativeLibStub(), lg, ArenaLogLevel.INFO)
    try:
        assert bridge._logger_for("arena.orders") is lg
    finally:
        bridge.restore_logger_configuration()


def test_restore_logger_configuration_factory_bridge_leaves_no_logger_to_restore():
    factory, _ = _recording_factory()
    bridge = _bridge_for_factory(factory)

    bridge.restore_logger_configuration()
