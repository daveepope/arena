"""Serialize readiness checks into FFI JSON (`readiness_checks` on exec/container).

Extension workflow (must match ``arena-ffi`` ``readiness_json`` / ``parse::ReadinessCheckJson``):

1. Add a Rust variant and dispatch in ``arena-ffi/src/readiness_json.rs``.
2. Add a branch in :func:`readiness_checks_for_ffi` for the Python type that maps to it.
3. Update other language clients similarly.

Only checks that have a defined JSON shape are included here; other
:class:`~arena_pytest.readiness.ReadinessCheck` implementations stay client-side
(see :meth:`arena_pytest.encounter.Encounter.readiness_hooks`).
"""

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
