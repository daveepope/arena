from __future__ import annotations

import asyncio
import json
from typing import List

from arena_pytest.arena import OpenArena
from arena_pytest.ffi._ffi import (
    ArenaFfiError,
    load_ffi,
    open_arena as ffi_open_arena,
)


class ClosedArena:
    def __init__(self, name: str, matches: List):
        self.name = name
        self._matches = matches

    def _config(self) -> str:
        if not self._matches:
            return "{}"
        a_match = self._matches[0]
        config = a_match._for_ffi() if hasattr(a_match, "_for_ffi") else a_match
        return json.dumps(config)

    async def open(self) -> OpenArena:
        ffi = load_ffi()
        if ffi is None:
            raise ArenaFfiError(
                "arena_ffi shared library not found. "
                "Build with: cargo build -p arena-ffi --release, "
                "or set ARENA_FFI_LIB to the .so path."
            )
        config = self._config()
        try:
            handle = await asyncio.to_thread(
                ffi_open_arena, ffi, self.name.encode("utf-8"), config
            )
        except ArenaFfiError as e:
            raise ArenaFfiError(f"arena_open failed: {e}") from e

        from arena_pytest.readiness import DEFAULT_READINESS_TIMEOUT_MS, run_readiness

        for m in self._matches:
            if not hasattr(m, "readiness_hooks"):
                continue
            for identifier, target, check in m.readiness_hooks():
                await run_readiness(check, identifier, target, DEFAULT_READINESS_TIMEOUT_MS)
        return OpenArena(ffi, handle)
