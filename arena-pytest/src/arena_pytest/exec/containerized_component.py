from typing import Any, Dict, List, Optional, Tuple

from arena_pytest.ffi._ffi_readiness import readiness_checks_for_ffi
from arena_pytest.support._identifier import build as _build_identifier
from arena_pytest.readiness import ReadinessCheck


class ContainerizedComponentBuilder:
    def __init__(self, name: str, containerfile: str):
        self._config: Dict[str, Any] = {
            "type": "container",
            "identifier": _build_identifier("arena-containerized-component", name),
            "containerfile": containerfile,
            "env_vars": {},
            "runtime_args": [],
            "port_mappings": [],
            "host_mappings": [],
        }
        self._readiness_checks: List[Tuple[ReadinessCheck, str]] = []

    def with_build_context(self, path: str) -> "ContainerizedComponentBuilder":
        self._config["build_context"] = path
        return self

    def with_image_tag(self, tag: str) -> "ContainerizedComponentBuilder":
        self._config["image_tag"] = tag
        return self

    def with_network(self, network: str) -> "ContainerizedComponentBuilder":
        self._config["network"] = network
        return self

    def with_port_mapping(self, host_port: int, container_port: int) -> "ContainerizedComponentBuilder":
        self._config["port_mappings"].append(
            {"host_port": host_port, "container_port": container_port}
        )
        return self

    def with_host_mapping(self, host_mapping: str) -> "ContainerizedComponentBuilder":
        self._config["host_mappings"].append(host_mapping)
        return self

    def with_env_var(self, key: str, value: str) -> "ContainerizedComponentBuilder":
        self._config["env_vars"][key] = value
        return self

    def with_runtime_arg(self, name: str, value: str) -> "ContainerizedComponentBuilder":
        self._config["runtime_args"].append({"name": name, "value": value})
        return self

    def with_readiness_check(self, check: ReadinessCheck, target: str) -> "ContainerizedComponentBuilder":
        self._readiness_checks.append((check, target))
        return self

    def build(self) -> "ContainerizedComponent":
        return ContainerizedComponent(dict(self._config), readiness_checks=list(self._readiness_checks))

    def _for_ffi(self) -> Dict[str, Any]:
        return dict(self._config)


class ContainerizedComponent:
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
