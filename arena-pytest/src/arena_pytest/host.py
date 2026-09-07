from __future__ import annotations

import ctypes
from enum import IntEnum

from arena_pytest.ffi._ffi import ArenaBindingError, load_ffi, _take_out_string

_ARENA_STATUS_OK = 0
_ARENA_STATUS_PANIC = 3


class PortSearchStrategy(IntEnum):
    RANDOM = 0
    LINEAR = 1


class ArenaPortNotFoundError(ArenaBindingError):
    pass


def find_available_port(
    start: int, end: int, strategy: PortSearchStrategy = PortSearchStrategy.RANDOM
) -> int:
    ffi = load_ffi()
    if ffi is None:
        raise ArenaBindingError(
            "arena_ffi shared library not found (required for find_available_port)"
        )
    port_out = ctypes.c_int32(0)
    err = ctypes.c_void_p()
    status = ffi.lib.arena_find_available_port(
        ctypes.c_int32(start),
        ctypes.c_int32(end),
        ctypes.c_int32(int(strategy)),
        ctypes.byref(port_out),
        ctypes.byref(err),
    )
    message = _take_out_string(err, ffi)
    if status == _ARENA_STATUS_PANIC:
        raise ArenaPortNotFoundError(message or "no available port found")
    if status != _ARENA_STATUS_OK:
        raise ArenaBindingError(message or f"find_available_port failed (status_code={status})")
    return port_out.value
