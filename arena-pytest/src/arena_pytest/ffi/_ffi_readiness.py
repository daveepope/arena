from __future__ import annotations

from typing import Any, Dict, List, Tuple, Union

from arena_pytest.readiness import HttpReadinessCheck, TcpReadinessCheck

ReadinessCheckEntry = Union[
    Tuple[Union[HttpReadinessCheck, TcpReadinessCheck], str],
    Tuple[Union[HttpReadinessCheck, TcpReadinessCheck], str, int],
]

_KIND_BY_CHECK_TYPE = {
    HttpReadinessCheck: "http",
    TcpReadinessCheck: "tcp",
}


def readiness_checks_for_ffi(
    readiness_checks: List[ReadinessCheckEntry],
) -> List[Dict[str, Any]]:
    out: List[Dict[str, Any]] = []
    for entry in readiness_checks:
        if len(entry) == 2:
            check, target = entry
            timeout_ms = 10_000
        else:
            check, target, timeout_ms = entry
        kind = _KIND_BY_CHECK_TYPE[type(check)]
        out.append({"kind": kind, "target": target, "timeout_ms": timeout_ms})
    return out
