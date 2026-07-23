from typing import Any, Dict

from arena_pytest.support._identifier import build as _build_identifier


class TemporalDependencyBuilder:
    def __init__(self, name: str = ""):
        self._config: Dict[str, Any] = {
            "type": "temporal",
            "identifier": _build_identifier("arena-temporal", name),
        }

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

    def build(self) -> "TemporalDependency":
        return TemporalDependency(dict(self._config))

    def _for_ffi(self) -> Dict[str, Any]:
        return dict(self._config)


class TemporalDependency:
    def __init__(self, config: Dict[str, Any]):
        self._config = config

    @property
    def identifier(self) -> str:
        return self._config["identifier"]

    def _for_ffi(self) -> Dict[str, Any]:
        return self._config
