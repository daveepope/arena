from __future__ import annotations

from collections import deque

from arena_pytest import ActivePlaybook, ManagedMssqlPlaybook, UnmanagedPlaybook

CALL_ORDER: deque[str] = deque()


class ResetProbePlaybook(ManagedMssqlPlaybook):
    def __init__(self, dependency_identifier: str):
        super().__init__(
            identifier="reset-timing-probe",
            dependency_identifier=dependency_identifier,
        )

    def run(self, arena) -> ActivePlaybook:
        active = super().run(arena)
        CALL_ORDER.append("managed")
        return active


class SeedProbePlaybook(UnmanagedPlaybook):
    def identifier(self) -> str:
        return "seed-timing-probe"

    def run(self, arena) -> ActivePlaybook:
        CALL_ORDER.append("unmanaged")
        return ActivePlaybook(None, 0)
