import asyncio
from typing import Any, Optional

import pytest

from arena_pytest._ffi import load_lib, open_arena, close_arena, soft_reset as ffi_soft_reset, hard_reset as ffi_hard_reset


class OpenArena:
    def __init__(self, lib: Any, handle: int):
        self._lib = lib
        self._handle = handle

    async def close(self) -> None:
        if self._handle is not None and self._handle != 0:
            await asyncio.to_thread(close_arena, self._lib, self._handle)
            self._handle = 0

    async def soft_reset(self, dependency_identifier: str) -> bool:
        """Soft reset a dependency (e.g. drop/recreate schema for Postgres, delete/recreate topics for Kafka)."""
        return await asyncio.to_thread(
            ffi_soft_reset, self._lib, self._handle, dependency_identifier
        )

    async def hard_reset(self, dependency_identifier: str) -> bool:
        """Hard reset a dependency (restart container, then run startup scripts / create topics)."""
        return await asyncio.to_thread(
            ffi_hard_reset, self._lib, self._handle, dependency_identifier
        )

    def is_valid(self) -> bool:
        return self._lib is not None and self._handle is not None and self._handle != 0


def get_arena_version() -> Optional[str]:
    lib = load_lib()
    if lib is None:
        return None
    result = lib.arena_ffi_version()
    return result.decode("utf-8").strip() if result else None


@pytest.fixture(scope="session")
def arena_lib():
    lib = load_lib()
    if lib is None:
        pytest.skip(
            "arena_ffi shared library not found. "
            "Build with: cargo build -p arena-ffi --release. "
            "Or set ARENA_FFI_LIB to the .so path."
        )
    return lib


@pytest.fixture(scope="session")
def closed_arena() -> Optional[Any]:
    return None


@pytest.fixture(scope="session")
async def arena(closed_arena) -> OpenArena:
    if closed_arena is None:
        pytest.skip("closed_arena fixture not overridden (no arena to open)")
    open_arena_obj = await closed_arena.open()
    if open_arena_obj is None or not open_arena_obj.is_valid():
        pytest.skip("arena_open failed (Docker required for dependencies)")
    yield open_arena_obj
    await open_arena_obj.close()
