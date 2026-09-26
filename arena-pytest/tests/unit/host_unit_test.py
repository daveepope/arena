import ctypes
from unittest.mock import MagicMock

import pytest

from arena_pytest.ffi._ffi import ArenaBindingError, ArenaNativeLib
from arena_pytest.host import ArenaPortNotFoundError, PortSearchStrategy, find_available_port
import arena_pytest.host as host_mod


def _fake_ffi(status: int, port: int = 0, message: bytes = b""):
    lib = MagicMock()

    def fake_find_available_port(start, end, strategy, port_out, err_out):
        port_out._obj.value = port
        if message:
            buf = ctypes.create_string_buffer(message)
            err_out._obj.value = ctypes.cast(buf, ctypes.c_void_p).value
            fake_find_available_port._keepalive = buf
        return status

    lib.arena_find_available_port.side_effect = fake_find_available_port
    lib.arena_free_string = MagicMock()
    return ArenaNativeLib(lib=lib)


def test_find_available_port_ok_status_returns_port(monkeypatch):
    ffi = _fake_ffi(status=0, port=12345)
    monkeypatch.setattr(host_mod, "load_ffi", lambda: ffi)

    port = find_available_port(20000, 21000, PortSearchStrategy.LINEAR)

    assert port == 12345


def test_find_available_port_panic_status_raises_port_not_found_error(monkeypatch):
    ffi = _fake_ffi(status=3, message=b"no available port found in range 1..2")
    monkeypatch.setattr(host_mod, "load_ffi", lambda: ffi)

    with pytest.raises(ArenaPortNotFoundError, match="no available port found"):
        find_available_port(1, 2)

    assert isinstance(ArenaPortNotFoundError("x"), ArenaBindingError)


def test_find_available_port_invalid_argument_status_raises_arena_binding_error(monkeypatch):
    ffi = _fake_ffi(status=1, message=b"range_start must be < range_end")
    monkeypatch.setattr(host_mod, "load_ffi", lambda: ffi)

    with pytest.raises(ArenaBindingError) as exc_info:
        find_available_port(5, 5)

    assert not isinstance(exc_info.value, ArenaPortNotFoundError)


def test_find_available_port_library_not_found_raises_arena_binding_error(monkeypatch):
    monkeypatch.setattr(host_mod, "load_ffi", lambda: None)

    with pytest.raises(ArenaBindingError, match="shared library not found"):
        find_available_port(1, 2)
