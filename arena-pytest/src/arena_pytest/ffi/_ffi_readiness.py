from __future__ import annotations

from typing import Any, Dict, List, Tuple

from arena_pytest.readiness import HttpReadinessCheck


def readiness_checks_for_ffi(
    readiness_checks: List[Tuple[HttpReadinessCheck, str]],
) -> List[Dict[str, Any]]:
    return [{"kind": "http", "target": target} for _, target in readiness_checks]
