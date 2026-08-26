from __future__ import annotations

from typing import Any, Dict, List


def children_for_ffi(children: List[Any]) -> List[Dict[str, Any]]:
    return [child._for_ffi() for child in children]
