from __future__ import annotations

import ctypes
import logging
import os
import sys
from dataclasses import dataclass
from enum import IntEnum
from typing import Any, Callable, Optional

class ArenaLogLevel(IntEnum):
    ERROR = 1
    WARN = 2
    INFO = 3
    DEBUG = 4
    TRACE = 5


_LOG_DISPATCHER_TRACE_NUM = logging.DEBUG - 2
logging.addLevelName(_LOG_DISPATCHER_TRACE_NUM, "TRACE")


class ArenaBindingError(RuntimeError):
    pass


def _ffi_expect_ok(raw: int, message: Optional[str], what_failed: str) -> None:
    if raw != 0:
        raise ArenaBindingError(message or f"{what_failed} (status_code={raw})")


@dataclass(frozen=True)
class ArenaNativeLib:
    lib: ctypes.CDLL


_ARENA_LOG_TARGET_CALLBACK_ABI = ctypes.CFUNCTYPE(
    None,
    ctypes.c_int,
    ctypes.c_void_p,
    ctypes.c_int64,
    ctypes.c_void_p,
    ctypes.c_void_p,
    ctypes.c_uint32,
    ctypes.c_void_p,
)

_ARENA_PY_GIL_ENSURE = ctypes.pythonapi.PyGILState_Ensure
_ARENA_PY_GIL_ENSURE.argtypes = []
_ARENA_PY_GIL_ENSURE.restype = ctypes.c_int

_ARENA_PY_GIL_RELEASE = ctypes.pythonapi.PyGILState_Release
_ARENA_PY_GIL_RELEASE.argtypes = [ctypes.c_int]
_ARENA_PY_GIL_RELEASE.restype = None

_default_dispatcher_logging_target_callback: Any = None

_DISPATCHER_DEFAULT_LOGGING_TARGET_LIB: Any = None


def _ffi_ptr_addr(raw: object) -> int:
    if raw is None:
        return 0
    try:
        v = ctypes.cast(raw, ctypes.c_void_p).value or 0
    except ctypes.ArgumentError:
        return 0
    return int(v)


def _utf8_zterm_at(addr: int) -> str:
    if addr == 0:
        return ""
    return ctypes.string_at(addr).decode("utf-8", errors="replace")


def _arena_dispatcher_logging_target_publish_level_for_python_logging(level: int) -> int:
    if level == ArenaLogLevel.ERROR:
        return logging.ERROR
    if level == ArenaLogLevel.WARN:
        return logging.WARNING
    if level == ArenaLogLevel.INFO:
        return logging.INFO
    if level == ArenaLogLevel.DEBUG:
        return logging.DEBUG
    if level == ArenaLogLevel.TRACE:
        return _LOG_DISPATCHER_TRACE_NUM
    return logging.INFO


def _dispatcher_log_append_caller_suffix(
    message: str, caller_file_addr: int, caller_line: int
) -> str:
    if caller_file_addr == 0 or caller_line <= 0:
        return message
    file_only = _utf8_zterm_at(caller_file_addr)
    return f"{message} [{file_only}:{caller_line}]"


def _flush_handlers_for_logger(lg: logging.Logger) -> None:
    while lg is not None:
        for h in lg.handlers:
            try:
                flush = getattr(h, "flush", None)
                if callable(flush):
                    flush()
            except (OSError, RuntimeError):
                pass
        if not lg.propagate:
            break
        lg = lg.parent


_ARENA_ROOT_LOGGER_NAME = "arena"


def _logger_for_record_target(lib, target_ptr) -> logging.Logger:
    addr = _ffi_ptr_addr(target_ptr)
    name = _utf8_zterm_at(addr) if addr else ""
    if not name:
        return _dispatcher_default_logger(lib)
    return logging.getLogger(name)


def _default_dispatcher_logging_target_invoke(
    level: int,
    target_ptr,
    ignored_ts_ns: int,
    message_ptr,
    caller_file_ptr,
    caller_line: int,
    ignored_user,
) -> None:
    gil_state = _ARENA_PY_GIL_ENSURE()
    try:
        lib = _DISPATCHER_DEFAULT_LOGGING_TARGET_LIB
        if lib is None:
            return
        publish = int(lib.arena_dispatcher_default_logging_target_publish_level(int(level)))
        publish_py = _arena_dispatcher_logging_target_publish_level_for_python_logging(publish)
        lg = _logger_for_record_target(lib, target_ptr)
        msg_addr = _ffi_ptr_addr(message_ptr)
        text = _utf8_zterm_at(msg_addr)
        cf_addr = _ffi_ptr_addr(caller_file_ptr)
        text = _dispatcher_log_append_caller_suffix(text, cf_addr, int(caller_line))
        lg.log(publish_py, "%s", text)
    finally:
        _ARENA_PY_GIL_RELEASE(gil_state)


_custom_dispatcher_logging_targets: dict[int, "_UserDispatcherLoggerBridge"] = {}


class _UserDispatcherLoggerBridge:
    __slots__ = (
        "_ffi_lib",
        "_logger",
        "_logger_factory",
        "_loggers_by_name",
        "_saved_logger_level",
        "_closure",
    )

    def __init__(
        self,
        ffi: ArenaNativeLib,
        lg: Optional[logging.Logger],
        arena_log_level: ArenaLogLevel,
        logger_factory: Optional[Callable[[str], logging.Logger]] = None,
    ):
        self._ffi_lib = ffi.lib
        self._logger = lg
        self._logger_factory = logger_factory
        self._loggers_by_name: dict[str, logging.Logger] = {}
        self._saved_logger_level = lg.level if lg is not None else None
        if lg is not None:
            _install_dispatcher_direct_stderr_emitter(lg, arena_log_level)
        self._closure = _ARENA_LOG_TARGET_CALLBACK_ABI(self._invoke)

    def _logger_for(self, logger_name: str) -> logging.Logger:
        if self._logger_factory is None:
            return self._logger
        name = logger_name or _ARENA_ROOT_LOGGER_NAME
        cached = self._loggers_by_name.get(name)
        if cached is None:
            cached = self._logger_factory(name)
            self._loggers_by_name[name] = cached
        return cached

    def _invoke(
        self,
        level: int,
        target_ptr,
        _ignored_ts_ns: int,
        message_ptr,
        caller_file_ptr,
        caller_line: int,
        _ignored_user_data,
    ) -> None:
        gil_state = _ARENA_PY_GIL_ENSURE()
        try:
            publish = int(
                self._ffi_lib.arena_dispatcher_default_logging_target_publish_level(int(level))
            )
            publish_py = _arena_dispatcher_logging_target_publish_level_for_python_logging(
                publish
            )
            target_addr = _ffi_ptr_addr(target_ptr)
            logger_name = _utf8_zterm_at(target_addr) if target_addr else ""
            lg = self._logger_for(logger_name)
            msg_addr = _ffi_ptr_addr(message_ptr)
            text = _utf8_zterm_at(msg_addr)
            cf_addr = _ffi_ptr_addr(caller_file_ptr)
            text = _dispatcher_log_append_caller_suffix(text, cf_addr, int(caller_line))
            if self._logger_factory is None and logger_name:
                text = f"{logger_name}  {text}"
            lg.log(publish_py, "%s", text)
        finally:
            _ARENA_PY_GIL_RELEASE(gil_state)

    def ffi_callback(self) -> Any:
        return self._closure

    def restore_logger_configuration(self) -> None:
        if self._logger is None:
            return
        _remove_dispatcher_direct_stderr_emitter(self._logger)
        self._logger.setLevel(self._saved_logger_level)


def set_dispatcher_dependency_allow_json(
    ffi: ArenaNativeLib, json_utf8: Optional[str]
) -> None:
    if json_utf8 is None:
        ffi.lib.arena_dispatcher_dependency_allow_json_set(None)
        return
    buf = ctypes.create_string_buffer(json_utf8.encode("utf-8"))
    ffi.lib.arena_dispatcher_dependency_allow_json_set(buf)


def set_dispatcher_component_allow_json(
    ffi: ArenaNativeLib, json_utf8: Optional[str]
) -> None:
    if json_utf8 is None:
        ffi.lib.arena_dispatcher_component_allow_json_set(None)
        return
    buf = ctypes.create_string_buffer(json_utf8.encode("utf-8"))
    ffi.lib.arena_dispatcher_component_allow_json_set(buf)


def _dispatcher_default_logger(lib) -> logging.Logger:
    nm_raw = lib.arena_dispatcher_default_logging_target_logger_name_utf8()
    nm_addr = _ffi_ptr_addr(nm_raw)
    name = (
        _utf8_zterm_at(nm_addr)
        if nm_addr
        else "arena"
    )
    return logging.getLogger(name)


def _logging_floor_level_for_arena(arena_level: ArenaLogLevel) -> int:
    if arena_level == ArenaLogLevel.ERROR:
        return logging.ERROR
    if arena_level == ArenaLogLevel.WARN:
        return logging.WARNING
    if arena_level == ArenaLogLevel.INFO:
        return logging.INFO
    if arena_level == ArenaLogLevel.DEBUG:
        return logging.DEBUG
    return _LOG_DISPATCHER_TRACE_NUM


def _arena_dispatcher_stderr_handler_predicate(handler: logging.Handler) -> bool:
    return getattr(handler, "_arena_pytest_dispatcher_stderr", False) is True


def _install_dispatcher_direct_stderr_emitter(lg: logging.Logger, arena_level: ArenaLogLevel) -> None:
    lg.setLevel(_logging_floor_level_for_arena(arena_level))
    for h in lg.handlers:
        if _arena_dispatcher_stderr_handler_predicate(h):
            return
    setattr(lg, "_arena_pytest_prev_propagate", lg.propagate)
    lg.propagate = False
    h = logging.StreamHandler(stream=sys.stderr)
    setattr(h, "_arena_pytest_dispatcher_stderr", True)
    h.setLevel(logging.NOTSET)
    h.setFormatter(
        logging.Formatter(
            fmt="%(asctime)s [%(process)d] %(levelname)s %(name)s - %(message)s",
            datefmt="%H:%M:%S",
        )
    )
    lg.addHandler(h)


def _remove_dispatcher_direct_stderr_emitter(lg: logging.Logger) -> None:
    for h in [x for x in list(lg.handlers) if _arena_dispatcher_stderr_handler_predicate(x)]:
        lg.removeHandler(h)
        try:
            h.close()
        except (OSError, RuntimeError):
            pass
    if hasattr(lg, "_arena_pytest_prev_propagate"):
        lg.propagate = bool(getattr(lg, "_arena_pytest_prev_propagate"))
        delattr(lg, "_arena_pytest_prev_propagate")


def register_dispatcher_logging_target_for_logger(
    ffi: ArenaNativeLib,
    logger: logging.Logger,
    *,
    arena_log_level: ArenaLogLevel = ArenaLogLevel.INFO,
) -> int:
    if logger is None:
        raise TypeError("logger must not be None")
    bridge = _UserDispatcherLoggerBridge(ffi, logger, arena_log_level)
    return _open_custom_dispatcher_logging_target(ffi, bridge)


def register_dispatcher_logging_target_for_logger_factory(
    ffi: ArenaNativeLib,
    logger_factory: Callable[[str], logging.Logger],
    *,
    arena_log_level: ArenaLogLevel = ArenaLogLevel.INFO,
) -> int:
    if logger_factory is None:
        raise TypeError("logger_factory must not be None")
    bridge = _UserDispatcherLoggerBridge(
        ffi, None, arena_log_level, logger_factory=logger_factory
    )
    return _open_custom_dispatcher_logging_target(ffi, bridge)


def _open_custom_dispatcher_logging_target(
    ffi: ArenaNativeLib, bridge: "_UserDispatcherLoggerBridge"
) -> int:
    token = int(ffi.lib.arena_add_log_target(bridge.ffi_callback(), ctypes.c_void_p()))
    if token == 0:
        bridge.restore_logger_configuration()
        raise ArenaBindingError("arena_add_log_target rejected callback")
    _custom_dispatcher_logging_targets[token] = bridge
    return token


def register_default_dispatcher_logging_target(
    ffi: ArenaNativeLib,
    *,
    arena_log_level: ArenaLogLevel = ArenaLogLevel.INFO,
) -> int:
    global _default_dispatcher_logging_target_callback, _DISPATCHER_DEFAULT_LOGGING_TARGET_LIB
    if _default_dispatcher_logging_target_callback is None:
        _default_dispatcher_logging_target_callback = _ARENA_LOG_TARGET_CALLBACK_ABI(
            _default_dispatcher_logging_target_invoke
        )
    _DISPATCHER_DEFAULT_LOGGING_TARGET_LIB = ffi.lib
    lg = _dispatcher_default_logger(ffi.lib)
    _install_dispatcher_direct_stderr_emitter(lg, arena_log_level)
    try:
        token = int(
            ffi.lib.arena_add_log_target(
                _default_dispatcher_logging_target_callback, ctypes.c_void_p()
            )
        )
        if token == 0:
            raise ArenaBindingError("arena_add_log_target rejected callback")
        return token
    except BaseException:
        _remove_dispatcher_direct_stderr_emitter(lg)
        raise


def unregister_dispatcher_logging_target(ffi: ArenaNativeLib, token: int) -> None:
    if not token:
        return
    bridge = _custom_dispatcher_logging_targets.pop(token, None)
    ffi.lib.arena_remove_log_target(ctypes.c_uint64(token))
    if bridge is not None:
        bridge.restore_logger_configuration()
        return
    _remove_dispatcher_direct_stderr_emitter(_dispatcher_default_logger(ffi.lib))


def find_lib() -> Optional[str]:
    path = os.environ.get("ARENA_FFI_LIB")
    if path and os.path.isfile(path):
        return path

    _ffi_module_dir = os.path.dirname(os.path.abspath(__file__))
    _pkg_dir = os.path.dirname(_ffi_module_dir)
    for name in (
        "libarena_ffi_shared.so",
        "libarena_ffi_shared.dylib",
        "arena_ffi_shared.dll",
    ):
        p = os.path.join(_pkg_dir, name)
        if os.path.isfile(p):
            return p

    try:
        from bazel_tools.tools.python.runfiles import runfiles

        r = runfiles.Create()
        for rel in (
            "arena/arena-ffi/libarena_ffi_shared.so",
            "arena/arena-ffi/libarena_ffi_shared.dylib",
            "arena/arena-ffi/arena_ffi_shared.dll",
            "_main/arena-ffi/libarena_ffi_shared.so",
            "_main/arena-ffi/libarena_ffi_shared.dylib",
            "_main/arena-ffi/arena_ffi_shared.dll",
            "arena/arena-ffi/libarena_ffi.so",
        ):
            p = r.Rlocation(rel)
            if p and os.path.isfile(p):
                return p
    except ImportError:
        pass

    runfiles_dir = os.environ.get("RUNFILES_DIR")
    if runfiles_dir:
        for subdir in ("_main/arena-ffi", "arena-ffi", ""):
            for name in (
                "libarena_ffi_shared.so",
                "libarena_ffi_shared.dylib",
                "libarena_ffi.so",
                "libarena_ffi.dylib",
                "arena_ffi_shared.dll",
                "arena_ffi.dll",
            ):
                p = (
                    os.path.join(runfiles_dir, subdir, name)
                    if subdir
                    else os.path.join(runfiles_dir, name)
                )
                if os.path.isfile(p):
                    return p

    arena_pytest_root = os.path.dirname(
        os.path.dirname(
            os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
        )
    )
    workspace_root = os.path.dirname(arena_pytest_root)
    for root in (os.getcwd(), arena_pytest_root, workspace_root):
        candidates = [
            os.path.join(root, "target", "release", "libarena_ffi.so"),
            os.path.join(root, "target", "release", "libarena_ffi.dylib"),
            os.path.join(root, "target", "release", "arena_ffi.dll"),
        ]
        for p in candidates:
            if os.path.isfile(p):
                return p
        deps = os.path.join(root, "target", "release", "deps")
        if os.path.isdir(deps):
            for name in os.listdir(deps):
                if (
                    name.startswith("libarena_ffi")
                    and (name.endswith(".so") or name.endswith(".dylib"))
                    or name.startswith("arena_ffi")
                    and name.endswith(".dll")
                ):
                    return os.path.join(deps, name)

    return None


def load_ffi() -> Optional[ArenaNativeLib]:
    path = find_lib()
    if not path:
        return None
    lib = ctypes.CDLL(path)

    lib.arena_open.argtypes = [
        ctypes.c_char_p,
        ctypes.c_char_p,
        ctypes.POINTER(ctypes.c_void_p),
        ctypes.POINTER(ctypes.c_void_p),
    ]
    lib.arena_open.restype = ctypes.c_void_p

    lib.arena_close.argtypes = [
        ctypes.c_void_p,
        ctypes.POINTER(ctypes.c_void_p),
        ctypes.POINTER(ctypes.c_void_p),
    ]
    lib.arena_close.restype = ctypes.c_int

    lib.arena_soft_reset.argtypes = [
        ctypes.c_void_p,
        ctypes.c_char_p,
        ctypes.POINTER(ctypes.c_void_p),
    ]
    lib.arena_soft_reset.restype = ctypes.c_int

    lib.arena_hard_reset.argtypes = [
        ctypes.c_void_p,
        ctypes.c_char_p,
        ctypes.POINTER(ctypes.c_void_p),
    ]
    lib.arena_hard_reset.restype = ctypes.c_int

    lib.arena_find_available_port.argtypes = [
        ctypes.c_int32,
        ctypes.c_int32,
        ctypes.c_int32,
        ctypes.POINTER(ctypes.c_int32),
        ctypes.POINTER(ctypes.c_void_p),
    ]
    lib.arena_find_available_port.restype = ctypes.c_int

    lib.arena_set_log_level.argtypes = [ctypes.c_int]
    lib.arena_set_log_level.restype = ctypes.c_int

    lib.arena_free_string.argtypes = [ctypes.c_void_p]

    lib.arena_add_log_target.argtypes = [_ARENA_LOG_TARGET_CALLBACK_ABI, ctypes.c_void_p]
    lib.arena_add_log_target.restype = ctypes.c_uint64

    lib.arena_remove_log_target.argtypes = [ctypes.c_uint64]
    lib.arena_remove_log_target.restype = None

    lib.arena_dispatcher_default_logging_target_logger_name_utf8.argtypes = []
    lib.arena_dispatcher_default_logging_target_logger_name_utf8.restype = ctypes.c_void_p

    lib.arena_dispatcher_default_logging_target_publish_level.argtypes = [
        ctypes.c_int
    ]
    lib.arena_dispatcher_default_logging_target_publish_level.restype = ctypes.c_int

    lib.arena_dispatcher_dependency_allow_json_set.argtypes = [
        ctypes.c_char_p,
    ]
    lib.arena_dispatcher_dependency_allow_json_set.restype = None

    lib.arena_dispatcher_component_allow_json_set.argtypes = [
        ctypes.c_char_p,
    ]
    lib.arena_dispatcher_component_allow_json_set.restype = None

    lib.arena_free_string.restype = None

    lib.arena_oauth_loopback_tls_pem_json.argtypes = [ctypes.POINTER(ctypes.c_void_p)]
    lib.arena_oauth_loopback_tls_pem_json.restype = ctypes.c_void_p

    lib.arena_oauth_sign_claims.argtypes = [
        ctypes.c_void_p,
        ctypes.c_char_p,
        ctypes.c_char_p,
        ctypes.c_char_p,
        ctypes.POINTER(ctypes.c_void_p),
    ]
    lib.arena_oauth_sign_claims.restype = ctypes.c_void_p

    lib.arena_match_playbook_run.argtypes = [
        ctypes.c_void_p,
        ctypes.c_char_p,
        ctypes.POINTER(ctypes.c_void_p),
    ]
    lib.arena_match_playbook_run.restype = ctypes.c_void_p

    lib.arena_active_playbook_drop.argtypes = [
        ctypes.c_void_p,
        ctypes.POINTER(ctypes.c_void_p),
    ]
    lib.arena_active_playbook_drop.restype = ctypes.c_int

    lib.arena_http_playbook_open.argtypes = [
        ctypes.c_void_p,
        ctypes.c_char_p,
        ctypes.POINTER(ctypes.c_void_p),
    ]
    lib.arena_http_playbook_open.restype = ctypes.c_void_p

    lib.arena_http_playbook_verify.argtypes = [
        ctypes.c_void_p,
        ctypes.c_char_p,
        ctypes.POINTER(ctypes.c_void_p),
    ]
    lib.arena_http_playbook_verify.restype = ctypes.c_int

    lib.arena_mssql_playbook_verify.argtypes = [
        ctypes.c_void_p,
        ctypes.c_char_p,
        ctypes.POINTER(ctypes.c_void_p),
    ]
    lib.arena_mssql_playbook_verify.restype = ctypes.c_int

    lib.arena_postgres_playbook_verify.argtypes = [
        ctypes.c_void_p,
        ctypes.c_char_p,
        ctypes.POINTER(ctypes.c_void_p),
    ]
    lib.arena_postgres_playbook_verify.restype = ctypes.c_int

    lib.arena_oracle_playbook_verify.argtypes = [
        ctypes.c_void_p,
        ctypes.c_char_p,
        ctypes.POINTER(ctypes.c_void_p),
    ]
    lib.arena_oracle_playbook_verify.restype = ctypes.c_int

    return ArenaNativeLib(lib=lib)


def _take_out_string(slot: "ctypes.c_void_p", ffi: ArenaNativeLib) -> Optional[str]:
    raw_ptr = slot.value
    if not raw_ptr:
        return None
    value = ctypes.string_at(raw_ptr).decode("utf-8", errors="replace")
    ffi.lib.arena_free_string(raw_ptr)
    slot.value = None
    return value


def open_arena(
    ffi: ArenaNativeLib,
    name: bytes = b"pytest-arena",
    config: Optional[str] = None,
    *,
    log_level: ArenaLogLevel = ArenaLogLevel.INFO,
) -> int:
    ffi.lib.arena_set_log_level(int(log_level))
    config_ptr = (config.encode("utf-8") + b"\0") if config else None
    err = ctypes.c_void_p()
    state = ctypes.c_void_p()
    handle = ffi.lib.arena_open(name, config_ptr, ctypes.byref(err), ctypes.byref(state))
    _take_out_string(state, ffi)
    if not handle:
        message = _take_out_string(err, ffi) or "arena_open returned null"
        raise ArenaBindingError(message)
    return handle


def close_arena(
    ffi: ArenaNativeLib,
    handle: int,
    *,
    dispatcher_logging_target_token: int = 0,
) -> None:
    if handle:
        err = ctypes.c_void_p()
        state = ctypes.c_void_p()
        ffi.lib.arena_close(handle, ctypes.byref(err), ctypes.byref(state))
        _take_out_string(err, ffi)
        _take_out_string(state, ffi)
    flush_lg = _dispatcher_default_logger(ffi.lib)
    if dispatcher_logging_target_token:
        bridge = _custom_dispatcher_logging_targets.get(dispatcher_logging_target_token)
        if bridge is not None:
            flush_lg = bridge._logger
        _flush_handlers_for_logger(flush_lg)
        unregister_dispatcher_logging_target(ffi, dispatcher_logging_target_token)
    else:
        _flush_handlers_for_logger(flush_lg)


def _reset(
    ffi: ArenaNativeLib,
    reset_fn,
    handle: int,
    dependency_identifier: str,
) -> None:
    if not handle:
        raise ArenaBindingError("reset called on closed arena")
    err = ctypes.c_void_p()
    raw = reset_fn(handle, dependency_identifier.encode("utf-8"), ctypes.byref(err))
    msg = _take_out_string(err, ffi)
    _ffi_expect_ok(raw, msg, "reset")


def soft_reset(ffi: ArenaNativeLib, handle: int, dependency_identifier: str) -> None:
    _reset(ffi, ffi.lib.arena_soft_reset, handle, dependency_identifier)


def hard_reset(ffi: ArenaNativeLib, handle: int, dependency_identifier: str) -> None:
    _reset(ffi, ffi.lib.arena_hard_reset, handle, dependency_identifier)


def oauth_sign_claims(
    ffi: ArenaNativeLib,
    handle: int,
    dependency_identifier: str,
    provider_json: str,
    claims_json: str,
) -> str:
    if not handle:
        raise ArenaBindingError("oauth_sign_claims called on closed arena")
    err = ctypes.c_void_p()
    raw = ffi.lib.arena_oauth_sign_claims(
        handle,
        dependency_identifier.encode("utf-8"),
        provider_json.encode("utf-8"),
        claims_json.encode("utf-8"),
        ctypes.byref(err),
    )
    if not raw:
        msg = _take_out_string(err, ffi) or "arena_oauth_sign_claims returned null"
        raise ArenaBindingError(msg)
    try:
        return ctypes.string_at(raw).decode("utf-8")
    finally:
        ffi.lib.arena_free_string(raw)


def match_playbook_run(
    ffi: ArenaNativeLib,
    arena_handle: int,
    identifier: str,
) -> int:
    if not arena_handle:
        raise ArenaBindingError("match_playbook_run called on closed arena")
    err = ctypes.c_void_p()
    pb_handle = ffi.lib.arena_match_playbook_run(
        arena_handle,
        identifier.encode("utf-8"),
        ctypes.byref(err),
    )
    message = _take_out_string(err, ffi)
    if not pb_handle:
        raise ArenaBindingError(message or "arena_match_playbook_run returned null")
    return pb_handle


def active_playbook_drop(ffi: ArenaNativeLib, handle: int) -> None:
    if not handle:
        return
    err = ctypes.c_void_p()
    raw = ffi.lib.arena_active_playbook_drop(handle, ctypes.byref(err))
    message = _take_out_string(err, ffi)
    _ffi_expect_ok(raw, message, "active_playbook_drop")


def http_playbook_open(
    ffi: ArenaNativeLib,
    arena_handle: int,
    open_spec_json: str,
) -> int:
    if not arena_handle:
        raise ArenaBindingError("http_playbook_open called on closed arena")
    err = ctypes.c_void_p()
    pb_handle = ffi.lib.arena_http_playbook_open(
        arena_handle,
        open_spec_json.encode("utf-8"),
        ctypes.byref(err),
    )
    message = _take_out_string(err, ffi)
    if not pb_handle:
        raise ArenaBindingError(message or "arena_http_playbook_open returned null")
    return pb_handle


def http_playbook_verify(
    ffi: ArenaNativeLib,
    handle: int,
    verify_spec_json: str,
) -> None:
    if not handle:
        raise ArenaBindingError(
            "http_playbook_verify called without an active playbook handle"
        )
    err = ctypes.c_void_p()
    raw = ffi.lib.arena_http_playbook_verify(
        handle,
        verify_spec_json.encode("utf-8"),
        ctypes.byref(err),
    )
    message = _take_out_string(err, ffi)
    _ffi_expect_ok(raw, message, "http_playbook_verify")


def mssql_playbook_verify(
    ffi: ArenaNativeLib,
    handle: int,
    verify_spec_json: str,
) -> None:
    if not handle:
        raise ArenaBindingError(
            "mssql_playbook_verify called without an active playbook handle"
        )
    err = ctypes.c_void_p()
    raw = ffi.lib.arena_mssql_playbook_verify(
        handle,
        verify_spec_json.encode("utf-8"),
        ctypes.byref(err),
    )
    message = _take_out_string(err, ffi)
    _ffi_expect_ok(raw, message, "mssql_playbook_verify")


def postgres_playbook_verify(
    ffi: ArenaNativeLib,
    handle: int,
    verify_spec_json: str,
) -> None:
    if not handle:
        raise ArenaBindingError(
            "postgres_playbook_verify called without an active playbook handle"
        )
    err = ctypes.c_void_p()
    raw = ffi.lib.arena_postgres_playbook_verify(
        handle,
        verify_spec_json.encode("utf-8"),
        ctypes.byref(err),
    )
    message = _take_out_string(err, ffi)
    _ffi_expect_ok(raw, message, "postgres_playbook_verify")


def oracle_playbook_verify(
    ffi: ArenaNativeLib,
    handle: int,
    verify_spec_json: str,
) -> None:
    if not handle:
        raise ArenaBindingError(
            "oracle_playbook_verify called without an active playbook handle"
        )
    err = ctypes.c_void_p()
    raw = ffi.lib.arena_oracle_playbook_verify(
        handle,
        verify_spec_json.encode("utf-8"),
        ctypes.byref(err),
    )
    message = _take_out_string(err, ffi)
    _ffi_expect_ok(raw, message, "oracle_playbook_verify")