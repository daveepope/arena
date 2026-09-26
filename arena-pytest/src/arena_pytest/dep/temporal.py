from datetime import timedelta

from arena_pytest.support._expiry import _expiry_seconds

from typing import Any, Dict, List, Optional

from arena_pytest.ffi._ffi_children import children_for_ffi
from arena_pytest.support._identifier import build as _build_identifier


class TemporalDependencyBuilder:
    def __init__(self, name: str = ""):
        self._config: Dict[str, Any] = {
            "type": "temporal",
            "identifier": _build_identifier("arena-temporal", name),
        }
        self._children: List[Any] = []

    def with_expiry(self, expiry: timedelta) -> "TemporalDependencyBuilder":
        self._config["expiry_seconds"] = _expiry_seconds(expiry)
        return self

    def without_expiry(self) -> "TemporalDependencyBuilder":
        self._config["expiry_seconds"] = 0
        return self

    def with_image_name(self, image_name: str) -> "TemporalDependencyBuilder":
        self._config["image_name"] = image_name
        return self

    def with_image(self, image: str) -> "TemporalDependencyBuilder":
        self._config["image"] = image
        return self

    def with_port(self, port: int) -> "TemporalDependencyBuilder":
        self._config["port"] = port
        return self

    def with_ui_port(self, ui_port: int) -> "TemporalDependencyBuilder":
        self._config["ui_port"] = ui_port
        return self

    def with_container_name(self, name: str) -> "TemporalDependencyBuilder":
        self._config["container_name"] = name
        return self

    def with_child_dependencies(self, children: List[Any]) -> "TemporalDependencyBuilder":
        self._children.extend(children)
        return self

    def build(self) -> "TemporalDependency":
        return TemporalDependency(dict(self._config), children=list(self._children))

    def _for_ffi(self) -> Dict[str, Any]:
        d = dict(self._config)
        children = children_for_ffi(self._children)
        if children:
            d["children"] = children
        return d


class TemporalDependency:
    def __init__(self, config: Dict[str, Any], children: Optional[List[Any]] = None):
        self._config = config
        self._children = children or []

    @property
    def identifier(self) -> str:
        return self._config["identifier"]

    def _for_ffi(self) -> Dict[str, Any]:
        d = dict(self._config)
        children = children_for_ffi(self._children)
        if children:
            d["children"] = children
        return d
