from __future__ import annotations

import asyncio
import json
import logging
import os
from typing import Callable, List, Optional, Sequence

from arena_pytest.arena import OpenArena
from arena_pytest.lifecycle import as_lifecycle_error
from arena_pytest.ffi._ffi import (
    ArenaBindingError,
    ArenaLogLevel,
    close_arena,
    dashboard_ensure_running,
    load_ffi,
    open_arena as ffi_open_arena,
    register_dashboard_observer,
    register_default_dispatcher_logging_target,
    register_dispatcher_logging_target_for_logger,
    register_dispatcher_logging_target_for_logger_factory,
    register_lifecycle_observer,
    set_dispatcher_component_allow_json,
    set_dispatcher_dependency_allow_json,
    unregister_dispatcher_logging_target,
    unregister_lifecycle_observer,
)

_LOG = logging.getLogger(__name__)

_DEFAULT_DASHBOARD_PORT = 4873


def _normalize_optional_id_seq(
    seq: Optional[Sequence[str]],
) -> tuple[str, ...]:
    if not seq:
        return ()
    return tuple(str(x).strip() for x in seq if str(x).strip())


class LifecycleDashboardBuilder:
    def __init__(self, closed: "ClosedArena") -> None:
        self._closed = closed

    def with_cpu_profiler(self, component_id: str) -> "LifecycleDashboardBuilder":
        self._closed._cpu_profiled_components.append(component_id)
        return self

    def with_cpu_profile_limit_bytes(self, limit_bytes: int) -> "LifecycleDashboardBuilder":
        self._closed._cpu_profile_limit_bytes = limit_bytes
        return self

    def done(self) -> "ClosedArena":
        return self._closed

    async def open(self) -> OpenArena:
        __tracebackhide__ = True
        return await self._closed.open()


class ClosedArena:
    def __init__(
        self,
        name: str,
        matches: List,
        *,
        log_level: ArenaLogLevel = ArenaLogLevel.INFO,
        logger: Optional[logging.Logger] = None,
        logger_factory: Optional[Callable[[str], logging.Logger]] = None,
        log_component_ids: Optional[Sequence[str]] = None,
        log_dependency_ids: Optional[Sequence[str]] = None,
    ):
        self.name = name
        self._matches = matches
        self._log_level = log_level
        self._logger = logger
        self._logger_factory = logger_factory
        self._log_dependency_ids = _normalize_optional_id_seq(log_dependency_ids)
        self._log_component_ids = _normalize_optional_id_seq(log_component_ids)
        self._observers: List[Callable[[str], None]] = []
        self._dashboard_url: Optional[str] = None
        self._dashboard_port: int = _DEFAULT_DASHBOARD_PORT
        self._dashboard_search_dir: Optional[str] = None
        self._lifecycle_dashboard = False
        self._cpu_profiled_components: List[str] = []
        self._cpu_profile_limit_bytes: Optional[int] = None

    def observe(self, callback: Callable[[str], None]) -> "ClosedArena":
        self._observers.append(callback)
        return self

    def with_lifecycle_dashboard(
        self,
        url: Optional[str] = None,
        *,
        port: int = _DEFAULT_DASHBOARD_PORT,
        search_dir: Optional[str] = None,
    ) -> "LifecycleDashboardBuilder":
        self._lifecycle_dashboard = True
        self._dashboard_url = url
        self._dashboard_port = port
        self._dashboard_search_dir = search_dir
        return LifecycleDashboardBuilder(self)

    async def _register_dashboard(self, ffi) -> Optional[int]:
        if not self._lifecycle_dashboard:
            return None
        try:
            dashboard_url = self._dashboard_url
            if not dashboard_url:
                search_dir = self._dashboard_search_dir or os.path.dirname(
                    os.path.abspath(__file__)
                )
                dashboard_url = await asyncio.to_thread(
                    dashboard_ensure_running, ffi, self._dashboard_port, search_dir
                )
            return await asyncio.to_thread(register_dashboard_observer, ffi, dashboard_url)
        except ArenaBindingError:
            _LOG.warning("lifecycle dashboard unavailable, continuing without it", exc_info=True)
            return None

    def _observer_token_for(self, ffi, callback: Callable[[str], None]) -> int:
        def _filtered(document: str) -> None:
            try:
                arena_id = json.loads(document).get("id")
            except (ValueError, TypeError):
                return
            if arena_id == self.name:
                callback(document)

        return register_lifecycle_observer(ffi, _filtered)

    def _config(self) -> str:
        if not self._matches:
            return "{}"
        a_match = self._matches[0]
        config = a_match._for_ffi() if hasattr(a_match, "_for_ffi") else a_match
        if self._cpu_profiled_components or self._cpu_profile_limit_bytes is not None:
            config = dict(config)
        if self._cpu_profiled_components:
            config["cpu_profiled_components"] = list(self._cpu_profiled_components)
        if self._cpu_profile_limit_bytes is not None:
            config["cpu_profile_limit_bytes"] = self._cpu_profile_limit_bytes
        return json.dumps(config)

    async def open(self) -> OpenArena:
        __tracebackhide__ = True
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
        observer_tokens = []
        log_tok = None
        try:
            if self._logger_factory is not None:
                log_tok = await asyncio.to_thread(
                    register_dispatcher_logging_target_for_logger_factory,
                    ffi,
                    self._logger_factory,
                    arena_log_level=self._log_level,
                )
            elif self._logger is not None:
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
            dashboard_token = await self._register_dashboard(ffi)
            if dashboard_token:
                observer_tokens.append(dashboard_token)
            for callback in self._observers:
                observer_tokens.append(
                    await asyncio.to_thread(self._observer_token_for, ffi, callback)
                )
            observer_tokens = tuple(observer_tokens)
        except Exception:
            for token in observer_tokens:
                await asyncio.to_thread(unregister_lifecycle_observer, ffi, token)
            if log_tok is not None:
                await asyncio.to_thread(unregister_dispatcher_logging_target, ffi, log_tok)
            raise
        try:
            handle = await asyncio.to_thread(
                ffi_open_arena,
                ffi,
                self.name.encode("utf-8"),
                config,
                log_level=self._log_level,
            )
        except ArenaBindingError as e:
            for token in observer_tokens:
                await asyncio.to_thread(unregister_lifecycle_observer, ffi, token)
            await asyncio.to_thread(
                close_arena, ffi, 0, dispatcher_logging_target_token=log_tok
            )
            raise as_lifecycle_error(e) from None

        return OpenArena(ffi, handle, log_tok, lifecycle_observer_tokens=observer_tokens)
