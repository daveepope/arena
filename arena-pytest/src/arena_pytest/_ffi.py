import ctypes
import os
from typing import Optional

ArenaLib = ctypes.CDLL


def find_lib() -> Optional[str]:
    path = os.environ.get("ARENA_FFI_LIB")
    if path and os.path.isfile(path):
        return path

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
                p = os.path.join(runfiles_dir, subdir, name) if subdir else os.path.join(runfiles_dir, name)
                if os.path.isfile(p):
                    return p

    arena_pytest_root = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
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
                if name.startswith("libarena_ffi") and (
                    name.endswith(".so") or name.endswith(".dylib")
                ) or name.startswith("arena_ffi") and name.endswith(".dll"):
                    return os.path.join(deps, name)

    return None


def load_lib() -> Optional[ArenaLib]:
    path = find_lib()
    if not path:
        return None
    lib = ctypes.CDLL(path)
    lib.arena_ffi_version.restype = ctypes.c_char_p
    lib.arena_open.argtypes = [ctypes.c_char_p, ctypes.c_char_p]
    lib.arena_open.restype = ctypes.c_void_p
    lib.arena_close.argtypes = [ctypes.c_void_p]
    lib.arena_close.restype = None
    lib.arena_soft_reset.argtypes = [ctypes.c_void_p, ctypes.c_char_p]
    lib.arena_soft_reset.restype = ctypes.c_bool
    lib.arena_hard_reset.argtypes = [ctypes.c_void_p, ctypes.c_char_p]
    lib.arena_hard_reset.restype = ctypes.c_bool
    return lib


def open_arena(
    lib: ArenaLib,
    name: bytes = b"pytest-arena",
    config_json: Optional[str] = None,
) -> Optional[int]:
    config_ptr = (config_json.encode("utf-8") + b"\0") if config_json else None
    return lib.arena_open(name, config_ptr)


def close_arena(lib: ArenaLib, handle: int) -> None:
    if handle is not None and handle != 0:
        lib.arena_close(handle)


def soft_reset(lib: ArenaLib, handle: int, dependency_identifier: str) -> bool:
    if handle is None or handle == 0:
        return False
    return lib.arena_soft_reset(handle, dependency_identifier.encode("utf-8"))


def hard_reset(lib: ArenaLib, handle: int, dependency_identifier: str) -> bool:
    if handle is None or handle == 0:
        return False
    return lib.arena_hard_reset(handle, dependency_identifier.encode("utf-8"))
