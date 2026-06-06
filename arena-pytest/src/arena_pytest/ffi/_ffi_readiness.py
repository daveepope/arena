from __future__ import annotations

from typing import Any, Dict, List, Tuple, Union

from arena_pytest.readiness import HttpReadinessCheck

ReadinessCheckEntry = Union[
    Tuple[HttpReadinessCheck, str],
    Tuple[HttpReadinessCheck, str, int],
]


def readiness_checks_for_ffi(
    readiness_checks: List[ReadinessCheckEntry],
) -> List[Dict[str, Any]]:
    out: List[Dict[str, Any]] = []
    for entry in readiness_checks:
        if len(entry) == 2:
            _, target = entry
            timeout_ms = 10_000
        else:
            _, target, timeout_ms = entry
        out.append({"kind": "http", "target": target, "timeout_ms": timeout_ms})
    return out
