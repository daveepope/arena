from datetime import timedelta

from arena_pytest.support._expiry import _expiry_seconds

from typing import Any, Dict, List, Optional

from arena_pytest.ffi._ffi_children import children_for_ffi
from arena_pytest.support._identifier import build as _build_identifier


class SmtpDependencyBuilder:
    def __init__(self, name: str = ""):
        self._config: Dict[str, Any] = {
            "type": "smtp",
            "identifier": _build_identifier("arena-smtp", name),
        }
        self._children: List[Any] = []

    def with_expiry(self, expiry: timedelta) -> "SmtpDependencyBuilder":
        self._config["expiry_seconds"] = _expiry_seconds(expiry)
        return self

    def without_expiry(self) -> "SmtpDependencyBuilder":
        self._config["expiry_seconds"] = 0
        return self

    def with_image_name(self, image_name: str) -> "SmtpDependencyBuilder":
        self._config["image_name"] = image_name
        return self

    def with_image(self, image: str) -> "SmtpDependencyBuilder":
        self._config["image"] = image
        return self

    def with_port(self, port: int) -> "SmtpDependencyBuilder":
        self._config["port"] = port
        return self

    def with_ui_port(self, ui_port: int) -> "SmtpDependencyBuilder":
        self._config["ui_port"] = ui_port
        return self

    def with_container_name(self, name: str) -> "SmtpDependencyBuilder":
        self._config["container_name"] = name
        return self

    def with_starttls(self) -> "SmtpDependencyBuilder":
        self._config["tls_mode"] = "starttls"
        return self

    def with_implicit_tls(self) -> "SmtpDependencyBuilder":
        self._config["tls_mode"] = "implicit"
        return self

    def with_child_dependencies(self, children: List[Any]) -> "SmtpDependencyBuilder":
        self._children.extend(children)
        return self

    def build(self) -> "SmtpDependency":
        return SmtpDependency(dict(self._config), children=list(self._children))

    def _for_ffi(self) -> Dict[str, Any]:
        d = dict(self._config)
        children = children_for_ffi(self._children)
        if children:
            d["children"] = children
        return d


class SmtpDependency:
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
