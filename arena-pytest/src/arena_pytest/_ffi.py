from __future__ import annotations

import ctypes
import os
from dataclasses import dataclass
from enum import IntEnum
from typing import Optional

ArenaLib = ctypes.CDLL


class ArenaStatus(IntEnum):
    OK = 0
    INVALID_ARGUMENT = 1
    FAILED = 2
    PANIC = 3
    NOT_FOUND = 4


class ArenaFfiError(RuntimeError):
    def __init__(self, message: str, status: Optional[ArenaStatus] = None) -> None:
        super().__init__(message)
        self.status = status


@dataclass(frozen=True)
class ArenaFfi:
    lib: ArenaLib


def find_lib() -> Optional[str]:
    path = os.environ.get("ARENA_FFI_LIB")
    if path and os.path.isfile(path):
        return path

    _pkg_dir = os.path.dirname(os.path.abspath(__file__))
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
        os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
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


def load_ffi() -> Optional[ArenaFfi]:
    path = find_lib()
    if not path:
        return None
    lib = ctypes.CDLL(path)

    lib.arena_open.argtypes = [
        ctypes.c_char_p,
        ctypes.c_char_p,
        ctypes.POINTER(ctypes.c_void_p),
    ]
    lib.arena_open.restype = ctypes.c_void_p

    lib.arena_close.argtypes = [ctypes.c_void_p]
    lib.arena_close.restype = None

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

    lib.arena_free_string.argtypes = [ctypes.c_void_p]
    lib.arena_free_string.restype = None

    lib.arena_http_playbook_open.argtypes = [
        ctypes.c_void_p,
        ctypes.c_char_p,
        ctypes.POINTER(ctypes.c_void_p),
    ]
    lib.arena_http_playbook_open.restype = ctypes.c_void_p

    lib.arena_http_playbook_close.argtypes = [
        ctypes.c_void_p,
        ctypes.POINTER(ctypes.c_void_p),
    ]
    lib.arena_http_playbook_close.restype = ctypes.c_int

    lib.arena_http_playbook_verify.argtypes = [
        ctypes.c_void_p,
        ctypes.c_char_p,
        ctypes.POINTER(ctypes.c_void_p),
    ]
    lib.arena_http_playbook_verify.restype = ctypes.c_int

    return ArenaFfi(lib=lib)


def _take_err(err_slot: "ctypes.c_void_p", ffi: ArenaFfi) -> Optional[str]:
    raw_ptr = err_slot.value
    if not raw_ptr:
        return None
    message = ctypes.string_at(raw_ptr).decode("utf-8", errors="replace")
    ffi.lib.arena_free_string(raw_ptr)
    err_slot.value = None
    return message


def open_arena(
    ffi: ArenaFfi,
    name: bytes = b"pytest-arena",
    config: Optional[str] = None,
) -> int:
    config_ptr = (config.encode("utf-8") + b"\0") if config else None
    err = ctypes.c_void_p()
    handle = ffi.lib.arena_open(name, config_ptr, ctypes.byref(err))
    if not handle:
        message = _take_err(err, ffi) or "arena_open returned null"
        raise ArenaFfiError(message)
    return handle


def close_arena(ffi: ArenaFfi, handle: int) -> None:
    if handle:
        ffi.lib.arena_close(handle)


def _reset(
    ffi: ArenaFfi,
    reset_fn,
    handle: int,
    dependency_identifier: str,
) -> ArenaStatus:
    if not handle:
        raise ArenaFfiError("reset called on closed arena", ArenaStatus.INVALID_ARGUMENT)
    err = ctypes.c_void_p()
    raw = reset_fn(handle, dependency_identifier.encode("utf-8"), ctypes.byref(err))
    message = _take_err(err, ffi)
    try:
        status = ArenaStatus(raw)
    except ValueError:
        raise ArenaFfiError(
            message or f"reset returned unknown status code {raw}",
            ArenaStatus.FAILED,
        )
    if status is not ArenaStatus.OK:
        raise ArenaFfiError(message or f"reset failed with status {status.name}", status)
    return status


def soft_reset(ffi: ArenaFfi, handle: int, dependency_identifier: str) -> ArenaStatus:
    return _reset(ffi, ffi.lib.arena_soft_reset, handle, dependency_identifier)


def hard_reset(ffi: ArenaFfi, handle: int, dependency_identifier: str) -> ArenaStatus:
    return _reset(ffi, ffi.lib.arena_hard_reset, handle, dependency_identifier)


def http_playbook_open(
    ffi: ArenaFfi,
    arena_handle: int,
    spec_json: str,
) -> int:
    if not arena_handle:
        raise ArenaFfiError(
            "http_playbook_open called on closed arena",
            ArenaStatus.INVALID_ARGUMENT,
        )
    err = ctypes.c_void_p()
    pb_handle = ffi.lib.arena_http_playbook_open(
        arena_handle,
        spec_json.encode("utf-8"),
        ctypes.byref(err),
    )
    message = _take_err(err, ffi)
    if not pb_handle:
        raise ArenaFfiError(message or "arena_http_playbook_open returned null")
    return pb_handle


def http_playbook_close(ffi: ArenaFfi, pb_handle: int) -> None:
    if not pb_handle:
        return
    err = ctypes.c_void_p()
    raw = ffi.lib.arena_http_playbook_close(pb_handle, ctypes.byref(err))
    message = _take_err(err, ffi)
    try:
        status = ArenaStatus(raw)
    except ValueError:
        raise ArenaFfiError(
            message or f"http_playbook_close returned unknown status {raw}",
            ArenaStatus.FAILED,
        )
    if status is not ArenaStatus.OK:
        raise ArenaFfiError(
            message or f"http_playbook_close failed with status {status.name}",
            status,
        )


def http_playbook_verify(
    ffi: ArenaFfi,
    pb_handle: int,
    verify_spec_json: str,
) -> None:
    if not pb_handle:
        raise ArenaFfiError(
            "http_playbook_verify called on closed playbook",
            ArenaStatus.INVALID_ARGUMENT,
        )
    err = ctypes.c_void_p()
    raw = ffi.lib.arena_http_playbook_verify(
        pb_handle,
        verify_spec_json.encode("utf-8"),
        ctypes.byref(err),
    )
    message = _take_err(err, ffi)
    try:
        status = ArenaStatus(raw)
    except ValueError:
        raise ArenaFfiError(
            message or f"http_playbook_verify returned unknown status {raw}",
            ArenaStatus.FAILED,
        )
    if status is not ArenaStatus.OK:
        raise ArenaFfiError(
            message or f"http_playbook_verify failed with status {status.name}",
            status,
        )

