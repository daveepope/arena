"""Python mirror of ``arena::healthcheck::ReadinessCheck``.

FFI-backed checks are serialized via :mod:`arena_pytest.ffi._ffi_readiness`; others may run
in :meth:`arena_pytest.closed_arena.ClosedArena.open` after ``arena_open``.
"""

from __future__ import annotations

import asyncio
from typing import Protocol, runtime_checkable

DEFAULT_READINESS_TIMEOUT_MS = 10_000


@runtime_checkable
class ReadinessCheck(Protocol):
    async def is_ready(self, identifier: str, target: str, timeout_ms: int) -> None:
        """Return when ready; raise on failure (Rust: Result<(), String>)."""
        ...


async def run_readiness(
    check: ReadinessCheck,
    identifier: str,
    target: str,
    timeout_ms: int,
) -> None:
    await check.is_ready(identifier, target, timeout_ms)


class HttpReadinessCheck:
    """HTTP readiness; same contract as Rust ``HttpReadinessCheck`` / ``arena-ffi`` HTTP dispatch.

    Serialization for the shared JSON contract lives in :mod:`arena_pytest.ffi._ffi_readiness`.
    After serialization, Rust runs the check during arena start (same as native
    ``with_readiness_check``). :meth:`is_ready` here is for client-only checks not sent
    through the FFI (e.g. custom Python readiness).
    """

    def __init__(self) -> None:
        pass

    @classmethod
    def new(cls) -> "HttpReadinessCheck":
        return cls()

    async def is_ready(self, identifier: str, target: str, timeout_ms: int) -> None:
        from arena_pytest.readiness.http_wait import wait_for_http_ready

        await asyncio.to_thread(
            wait_for_http_ready,
            target,
            timeout_ms / 1000.0,
        )
