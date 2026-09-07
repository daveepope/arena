import signal
import sys

import pytest


def _arena_module():
    import arena_pytest.arena  # noqa: F401

    return sys.modules["arena_pytest.arena"]


def _reset(handler):
    signal.signal(signal.SIGTERM, handler)


def test_install_sigterm_teardown_default_handler_installs_ours():
    arena = _arena_module()
    original = signal.getsignal(signal.SIGTERM)
    _reset(signal.SIG_DFL)
    try:
        arena._install_sigterm_teardown()
        assert signal.getsignal(signal.SIGTERM) is arena._exit_session_on_sigterm
    finally:
        arena._previous_sigterm_handler = None
        _reset(original)


def test_install_sigterm_teardown_existing_handler_is_left_alone():
    arena = _arena_module()
    original = signal.getsignal(signal.SIGTERM)

    def existing(signum, frame):
        raise AssertionError("never called")

    _reset(existing)
    try:
        arena._install_sigterm_teardown()
        assert signal.getsignal(signal.SIGTERM) is existing
    finally:
        arena._previous_sigterm_handler = None
        _reset(original)


def test_restore_sigterm_handler_after_install_restores_default():
    arena = _arena_module()
    original = signal.getsignal(signal.SIGTERM)
    _reset(signal.SIG_DFL)
    try:
        arena._install_sigterm_teardown()
        arena._restore_sigterm_handler()
        assert signal.getsignal(signal.SIGTERM) == signal.SIG_DFL
        assert arena._previous_sigterm_handler is None
    finally:
        arena._previous_sigterm_handler = None
        _reset(original)


def test_exit_session_on_sigterm_sigterm_raises_exit_with_143():
    arena = _arena_module()

    with pytest.raises(pytest.exit.Exception) as raised:
        arena._exit_session_on_sigterm(signal.SIGTERM, None)

    assert raised.value.returncode == 143


def test_restore_sigterm_handler_when_ours_not_installed_is_a_noop():
    arena = _arena_module()
    original = signal.getsignal(signal.SIGTERM)

    def foreign(signum, frame):
        raise AssertionError("never called")

    _reset(foreign)
    try:
        arena._previous_sigterm_handler = None

        arena._restore_sigterm_handler()

        assert signal.getsignal(signal.SIGTERM) is foreign
    finally:
        arena._previous_sigterm_handler = None
        _reset(original)


def test_install_sigterm_teardown_called_twice_keeps_original_handler():
    arena = _arena_module()
    original = signal.getsignal(signal.SIGTERM)

    def existing(signum, frame):
        raise AssertionError("never called")

    _reset(existing)
    try:
        arena._previous_sigterm_handler = None
        signal.signal(signal.SIGTERM, signal.SIG_DFL)
        arena._install_sigterm_teardown()
        arena._install_sigterm_teardown()
        arena._restore_sigterm_handler()
        assert signal.getsignal(signal.SIGTERM) == signal.SIG_DFL
    finally:
        arena._previous_sigterm_handler = None
        _reset(original)


def test_restore_sigterm_handler_when_ours_was_replaced_leaves_replacement():
    arena = _arena_module()
    original = signal.getsignal(signal.SIGTERM)

    def other(signum, frame):
        raise AssertionError("never called")

    _reset(signal.SIG_DFL)
    try:
        arena._install_sigterm_teardown()
        signal.signal(signal.SIGTERM, other)
        arena._restore_sigterm_handler()
        assert signal.getsignal(signal.SIGTERM) is other
        assert arena._previous_sigterm_handler is None
    finally:
        arena._previous_sigterm_handler = None
        _reset(original)
