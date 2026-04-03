import asyncio
import json
from typing import List, Optional

from arena_pytest.arena import OpenArena
from arena_pytest._ffi import load_lib, open_arena as ffi_open_arena


class ClosedArena:
    def __init__(self, name: str, encounters: List):
        self.name = name
        self._encounters = encounters

    def _config_json(self) -> str:
        if not self._encounters:
            return "{}"
        encounter = self._encounters[0]
        config = encounter._for_ffi() if hasattr(encounter, "_for_ffi") else encounter
        return json.dumps(config)

    async def open(self) -> Optional[OpenArena]:
        import pytest

        lib = load_lib()
        if lib is None:
            pytest.skip(
                "arena_ffi shared library not found. "
                "Build with: cargo build -p arena-ffi --release. "
                "Or set ARENA_FFI_LIB to the .so path."
            )
        config_json = self._config_json()
        handle = await asyncio.to_thread(
            ffi_open_arena, lib, self.name.encode("utf-8"), config_json
        )
        if handle is None or handle == 0:
            return None
        from arena_pytest.readiness import DEFAULT_READINESS_TIMEOUT_MS, run_readiness

        for enc in self._encounters:
            if not hasattr(enc, "readiness_hooks"):
                continue
            for identifier, target, check in enc.readiness_hooks():
                await run_readiness(check, identifier, target, DEFAULT_READINESS_TIMEOUT_MS)
        return OpenArena(lib, handle)
