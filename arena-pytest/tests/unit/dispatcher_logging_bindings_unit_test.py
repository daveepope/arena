import ctypes
import logging
import sys

_TESTS_DIR = __import__("os").path.dirname(__import__("os").path.abspath(__file__))
if _TESTS_DIR not in sys.path:
    sys.path.insert(0, _TESTS_DIR)

import pytest

from arena_pytest.ffi._ffi import (
    ArenaLogLevel,
    load_ffi,
    register_default_dispatcher_logging_target,
    register_dispatcher_logging_target_for_logger,
    set_dispatcher_component_allow_json,
    set_dispatcher_dependency_allow_json,
    unregister_dispatcher_logging_target,
)

_MATCH_JSON_OPEN_FAILS_AT_BUILD_WITHOUT_CONTAINERS = (
    b'{"dependencies":[{"type":"kafka","identifier":"dispatcher-logging-pytest-host",'
    b'"flavor":"not_supported_for_dispatcher_host_logging_test"}]}'
)

_RESTRICTIVE_DISPATCHER_ALLOW_JSON = '["nope-artificial-needle-not-in-product"]'

class _LineCaptureHandler(logging.Handler):
    __slots__ = ("lines",)

    def __init__(self) -> None:
        super().__init__()
        self.lines: list[tuple[int, str]] = []

    def emit(self, record: logging.LogRecord) -> None:
        self.lines.append((record.levelno, record.getMessage()))


def _build_real_logger() -> tuple[logging.Logger, _LineCaptureHandler]:
    lg = logging.getLogger(f"arena.pytest.dispatcher.host.{id(object())}")
    lg.handlers.clear()
    lg.setLevel(logging.NOTSET)
    capture = _LineCaptureHandler()
    capture.setLevel(logging.NOTSET)
    lg.addHandler(capture)
    lg.propagate = True
    return lg, capture


def _arena_open_null_peer_and_err_text(
    ffi,
    *,
    arena_host_binding: bytes = b"pytest-dispatcher-host-logging-binding",
) -> tuple[int, str]:
    err_slot = ctypes.c_void_p()
    raw = ffi.lib.arena_open(
        ctypes.c_char_p(arena_host_binding),
        ctypes.c_char_p(_MATCH_JSON_OPEN_FAILS_AT_BUILD_WITHOUT_CONTAINERS),
        ctypes.byref(err_slot),
    )
    peer = ctypes.cast(raw, ctypes.c_void_p).value or 0
    text = ""
    if err_slot.value:
        text = ctypes.string_at(err_slot.value).decode("utf-8", errors="replace")
        ffi.lib.arena_free_string(err_slot.value)
    return peer, text


def _clear_dispatcher_allows(ffi) -> None:
    set_dispatcher_dependency_allow_json(ffi, None)
    set_dispatcher_component_allow_json(ffi, None)


@pytest.fixture
def arena_ffi():
    ffi = load_ffi()
    if ffi is None:
        pytest.skip("arena shared library not found")
    _clear_dispatcher_allows(ffi)
    yield ffi
    _clear_dispatcher_allows(ffi)


@pytest.mark.parametrize(
    ("ffi_level", "rust_level_token", "expect_info_carrier"),
    [
        pytest.param(ArenaLogLevel.ERROR, "Error", False, id="Error"),
        pytest.param(ArenaLogLevel.WARN, "Warn", False, id="Warn"),
        pytest.param(ArenaLogLevel.INFO, "Info", True, id="Info"),
        pytest.param(ArenaLogLevel.DEBUG, "Debug", True, id="Debug"),
        pytest.param(ArenaLogLevel.TRACE, "Trace", True, id="Trace"),
    ],
)
def test_register_custom_logger_arena_set_log_level_ffi_level_carrier_matches_floor(
    arena_ffi,
    ffi_level: ArenaLogLevel,
    rust_level_token: str,
    expect_info_carrier: bool,
) -> None:
    lg, capture = _build_real_logger()
    tok = register_dispatcher_logging_target_for_logger(
        arena_ffi, lg, arena_log_level=ArenaLogLevel.TRACE
    )
    try:
        capture.lines.clear()
        arena_ffi.lib.arena_set_log_level(int(ffi_level))
        if not expect_info_carrier:
            assert capture.lines == []
            return
        assert any(logging.INFO == lvl for lvl, _ in capture.lines), capture.lines
        assert any("arena log level set" in msg for _, msg in capture.lines), capture.lines
        needle = f"arena_log_level={rust_level_token}"
        assert any(needle in msg for _, msg in capture.lines), capture.lines
    finally:
        unregister_dispatcher_logging_target(arena_ffi, tok)


def test_register_custom_logger_arena_open_build_fail_inside_lib_forward_error(
    arena_ffi,
) -> None:
    lg, capture = _build_real_logger()
    tok = register_dispatcher_logging_target_for_logger(
        arena_ffi, lg, arena_log_level=ArenaLogLevel.TRACE
    )
    try:
        arena_ffi.lib.arena_set_log_level(int(ArenaLogLevel.TRACE))
        capture.lines.clear()
        peer, err_text = _arena_open_null_peer_and_err_text(arena_ffi)
        assert peer == 0
        assert "kafka flavor" in err_text.lower()
        errs = [(lvl, msg) for lvl, msg in capture.lines if logging.ERROR == lvl]
        assert errs, capture.lines
        assert any(
            ("open failed" in msg or "arena_open" in msg) for _, msg in errs
        ), capture.lines
    finally:
        unregister_dispatcher_logging_target(arena_ffi, tok)


def test_register_custom_logger_then_unregister_restores_stderr_and_propagate(
    arena_ffi,
) -> None:
    lg, _capture = _build_real_logger()
    assert lg.propagate is True
    tok = register_dispatcher_logging_target_for_logger(
        arena_ffi, lg, arena_log_level=ArenaLogLevel.INFO
    )
    assert lg.propagate is False
    assert any(
        getattr(h, "_arena_pytest_dispatcher_stderr", False) for h in lg.handlers
    )
    unregister_dispatcher_logging_target(arena_ffi, tok)
    assert lg.propagate is True
    assert not any(
        getattr(h, "_arena_pytest_dispatcher_stderr", False) for h in lg.handlers
    )


@pytest.mark.parametrize(
    ("ffi_level", "rust_level_token", "expect_info_carrier"),
    [
        pytest.param(ArenaLogLevel.ERROR, "Error", False, id="Error"),
        pytest.param(ArenaLogLevel.WARN, "Warn", False, id="Warn"),
        pytest.param(ArenaLogLevel.INFO, "Info", True, id="Info"),
        pytest.param(ArenaLogLevel.DEBUG, "Debug", True, id="Debug"),
        pytest.param(ArenaLogLevel.TRACE, "Trace", True, id="Trace"),
    ],
)
def test_register_default_target_arena_set_log_level_ffi_level_stderr_matches_floor(
    arena_ffi,
    capsys: pytest.CaptureFixture[str],
    ffi_level: ArenaLogLevel,
    rust_level_token: str,
    expect_info_carrier: bool,
) -> None:
    capsys.readouterr()
    tok = register_default_dispatcher_logging_target(
        arena_ffi, arena_log_level=ArenaLogLevel.TRACE
    )
    try:
        arena_ffi.lib.arena_set_log_level(int(ffi_level))
        err_out = capsys.readouterr().err
        if not expect_info_carrier:
            assert "arena log level set" not in err_out
            return
        assert "arena log level set" in err_out
        assert f"arena_log_level={rust_level_token}" in err_out
    finally:
        unregister_dispatcher_logging_target(arena_ffi, tok)


def test_register_custom_logger_restrictive_dispatcher_allows_still_forwards_arena_ffi_set_level(
    arena_ffi,
) -> None:
    set_dispatcher_dependency_allow_json(arena_ffi, _RESTRICTIVE_DISPATCHER_ALLOW_JSON)
    set_dispatcher_component_allow_json(arena_ffi, _RESTRICTIVE_DISPATCHER_ALLOW_JSON)
    lg, capture = _build_real_logger()
    tok = register_dispatcher_logging_target_for_logger(
        arena_ffi, lg, arena_log_level=ArenaLogLevel.TRACE
    )
    try:
        capture.lines.clear()
        arena_ffi.lib.arena_set_log_level(int(ArenaLogLevel.INFO))
        assert any("arena log level set" in msg for _, msg in capture.lines), capture.lines
    finally:
        unregister_dispatcher_logging_target(arena_ffi, tok)


def test_register_custom_logger_restrictive_dispatcher_allows_still_forwards_arena_open_error(
    arena_ffi,
) -> None:
    set_dispatcher_dependency_allow_json(arena_ffi, _RESTRICTIVE_DISPATCHER_ALLOW_JSON)
    set_dispatcher_component_allow_json(arena_ffi, _RESTRICTIVE_DISPATCHER_ALLOW_JSON)
    lg, capture = _build_real_logger()
    tok = register_dispatcher_logging_target_for_logger(
        arena_ffi, lg, arena_log_level=ArenaLogLevel.TRACE
    )
    try:
        arena_ffi.lib.arena_set_log_level(int(ArenaLogLevel.TRACE))
        capture.lines.clear()
        peer, err_text = _arena_open_null_peer_and_err_text(
            arena_ffi,
            arena_host_binding=b"pytest-dispatcher-host-logging-binding-open-error",
        )
        assert peer == 0
        assert "kafka flavor" in err_text.lower()
        errs = [(lvl, msg) for lvl, msg in capture.lines if logging.ERROR == lvl]
        assert errs, capture.lines
        assert any(
            ("open failed" in msg or "arena_open" in msg) for _, msg in errs
        ), capture.lines
    finally:
        unregister_dispatcher_logging_target(arena_ffi, tok)


if __name__ == "__main__":
    sys.exit(
        pytest.main(
            [__import__("os").path.dirname(__import__("os").path.abspath(__file__)), "-v", "-s"]
        )
    )
