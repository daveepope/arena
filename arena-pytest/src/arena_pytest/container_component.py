from typing import Any, Dict, List, Optional, Tuple

from arena_pytest._ffi_readiness import readiness_checks_for_ffi
from arena_pytest.readiness import ReadinessCheck


class ContainerComponentBuilder:
    def __init__(self, identifier: str, dockerfile: str):
        self._config: Dict[str, Any] = {
            "type": "container",
            "identifier": identifier,
            "dockerfile": dockerfile,
            "env_vars": {},
            "runtime_args": [],
            "port_mappings": [],
        }
        self._readiness_checks: List[Tuple[ReadinessCheck, str]] = []

    def with_build_context(self, path: str) -> "ContainerComponentBuilder":
        self._config["build_context"] = path
        return self

    def with_image_tag(self, tag: str) -> "ContainerComponentBuilder":
        self._config["image_tag"] = tag
        return self

    def with_network(self, network: str) -> "ContainerComponentBuilder":
        self._config["network"] = network
        return self

    def with_port_mapping(self, host_port: int, container_port: int) -> "ContainerComponentBuilder":
        self._config["port_mappings"].append(
            {"host_port": host_port, "container_port": container_port}
        )
        return self

    def with_env_var(self, key: str, value: str) -> "ContainerComponentBuilder":
        self._config["env_vars"][key] = value
        return self

    def with_runtime_arg(self, name: str, value: str) -> "ContainerComponentBuilder":
        self._config["runtime_args"].append({"name": name, "value": value})
        return self

    def with_readiness_check(self, check: ReadinessCheck, target: str) -> "ContainerComponentBuilder":
        self._readiness_checks.append((check, target))
        return self

    def build(self) -> "ContainerComponent":
        return ContainerComponent(dict(self._config), readiness_checks=list(self._readiness_checks))

    def _for_ffi(self) -> Dict[str, Any]:
        return dict(self._config)


class ContainerComponent:
    def __init__(
        self,
        config: Dict[str, Any],
        readiness_checks: Optional[List[Tuple[ReadinessCheck, str]]] = None,
    ):
        self._config = config
        self._readiness_checks = readiness_checks or []

    def _for_ffi(self) -> Dict[str, Any]:
        d = dict(self._config)
        rc = readiness_checks_for_ffi(self._readiness_checks)
        if rc:
            d["readiness_checks"] = rc
        return d
