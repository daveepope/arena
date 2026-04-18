from __future__ import annotations

from typing import Any, Dict, List, Tuple

from arena_pytest.readiness import HttpReadinessCheck, ReadinessCheck


def readiness_checks_for_ffi(
    readiness_checks: List[Tuple[ReadinessCheck, str]],
) -> List[Dict[str, Any]]:
    out: List[Dict[str, Any]] = []
    for check, target in readiness_checks:
        if isinstance(check, HttpReadinessCheck):
            out.append({"kind": "http", "target": target})
    return out
