from __future__ import annotations

import asyncio
from typing import Any, Optional

import pytest
import pytest_asyncio

from arena_pytest.ffi._ffi import (
    ArenaBindingError,
    ArenaNativeLib,
    close_arena,
    hard_reset as ffi_hard_reset,
    load_ffi,
    soft_reset as ffi_soft_reset,
)


class OpenArena:
    def __init__(
        self,
        ffi: ArenaNativeLib,
        handle: int,
        dispatcher_logging_target_token: int = 0,
    ):
        self._ffi = ffi
        self._handle = handle
        self._dispatcher_logging_target_token = dispatcher_logging_target_token

    def handle(self) -> int:
        return self._handle

    async def close(self) -> None:
        if self._handle:
            await asyncio.to_thread(
                close_arena,
                self._ffi,
                self._handle,
                dispatcher_logging_target_token=self._dispatcher_logging_target_token,
            )
            self._handle = 0
            self._dispatcher_logging_target_token = 0

    async def soft_reset(self, dependency_identifier: str) -> None:
        await asyncio.to_thread(
            ffi_soft_reset, self._ffi, self._handle, dependency_identifier
        )

    async def hard_reset(self, dependency_identifier: str) -> None:
        await asyncio.to_thread(
            ffi_hard_reset, self._ffi, self._handle, dependency_identifier
        )


@pytest.fixture(scope="session")
def arena_ffi() -> ArenaNativeLib:
    ffi = load_ffi()
    if ffi is None:
        pytest.skip(
            "arena shared library not found (set ARENA_FFI_LIB or use Bazel runfiles)"
        )
    return ffi


@pytest.fixture(scope="session")
def closed_arena() -> Optional[Any]:
    return None


@pytest_asyncio.fixture(scope="session")
async def arena(closed_arena) -> OpenArena:
    if closed_arena is None:
        pytest.skip("closed_arena fixture not overridden (no arena to open)")
    try:
        open_arena_obj = await closed_arena.open()
    except ArenaBindingError as e:
        pytest.fail(f"arena_open failed: {e}", pytrace=False)
    yield open_arena_obj
    await open_arena_obj.close()
