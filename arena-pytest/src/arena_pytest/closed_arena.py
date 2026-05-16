from __future__ import annotations

import asyncio
import json
import logging
from typing import List, Optional, Sequence

from arena_pytest.arena import OpenArena
from arena_pytest.ffi._ffi import (
    ArenaBindingError,
    ArenaLogLevel,
    close_arena,
    load_ffi,
    open_arena as ffi_open_arena,
    register_default_dispatcher_logging_target,
    register_dispatcher_logging_target_for_logger,
    set_dispatcher_component_allow_json,
    set_dispatcher_dependency_allow_json,
)


def _normalize_optional_id_seq(
    seq: Optional[Sequence[str]],
) -> tuple[str, ...]:
    if not seq:
        return ()
    return tuple(str(x).strip() for x in seq if str(x).strip())


class ClosedArena:
    def __init__(
        self,
        name: str,
        matches: List,
        *,
        log_level: ArenaLogLevel = ArenaLogLevel.INFO,
        logger: Optional[logging.Logger] = None,
        log_component_ids: Optional[Sequence[str]] = None,
        log_dependency_ids: Optional[Sequence[str]] = None,
    ):
        self.name = name
        self._matches = matches
        self._log_level = log_level
        self._logger = logger
        self._log_dependency_ids = _normalize_optional_id_seq(log_dependency_ids)
        self._log_component_ids = _normalize_optional_id_seq(log_component_ids)

    def _config(self) -> str:
        if not self._matches:
            return "{}"
        a_match = self._matches[0]
        config = a_match._for_ffi() if hasattr(a_match, "_for_ffi") else a_match
        return json.dumps(config)

    async def open(self) -> OpenArena:
        ffi = load_ffi()
        if ffi is None:
            raise ArenaBindingError(
                "arena shared library not found (set ARENA_FFI_LIB or use Bazel runfiles)"
            )
        config = self._config()
        dep_json = (
            json.dumps(list(self._log_dependency_ids))
            if self._log_dependency_ids
            else None
        )
        comp_json = (
            json.dumps(list(self._log_component_ids))
            if self._log_component_ids
            else None
        )
        await asyncio.to_thread(set_dispatcher_dependency_allow_json, ffi, dep_json)
        await asyncio.to_thread(set_dispatcher_component_allow_json, ffi, comp_json)
        if self._logger is not None:
            log_tok = await asyncio.to_thread(
                register_dispatcher_logging_target_for_logger,
                ffi,
                self._logger,
                arena_log_level=self._log_level,
            )
        else:
            log_tok = await asyncio.to_thread(
                register_default_dispatcher_logging_target,
                ffi,
                arena_log_level=self._log_level,
            )
        try:
            handle = await asyncio.to_thread(
                ffi_open_arena,
                ffi,
                self.name.encode("utf-8"),
                config,
                log_level=self._log_level,
            )
        except ArenaBindingError as e:
            await asyncio.to_thread(
                close_arena, ffi, 0, dispatcher_logging_target_token=log_tok
            )
            raise ArenaBindingError(f"arena_open failed: {e}") from e

        return OpenArena(ffi, handle, log_tok)
