from enum import Enum
from typing import Any, Dict, List, Optional, Tuple, Union

from arena_pytest.ffi._ffi_children import children_for_ffi
from arena_pytest.ffi._ffi_readiness import ReadinessCheckEntry, readiness_checks_for_ffi
from arena_pytest.support._identifier import build as _build_identifier
from arena_pytest.readiness import HttpReadinessCheck, TcpReadinessCheck


class BuildTool(Enum):
    CARGO = "cargo"
    MAVEN = "maven"
    GRADLE = "gradle"
    DOTNET = "dotnet"
    MAKE = "make"
    CMAKE = "cmake"

    @staticmethod
    def custom(command: str, args: List[str]) -> Dict[str, Any]:
        return {"command": command, "args": args}


class ExecutableComponentBuilder:
    def __init__(self, name: str = ""):
        self._config: Dict[str, Any] = {
            "type": "exec",
            "identifier": _build_identifier("arena-executable-component", name),
            "env_vars": {},
            "runtime_args": [],
        }
        self._readiness_checks: List[ReadinessCheckEntry] = []
        self._children: List[Any] = []

    def with_executable_path(self, path: str) -> "ExecutableComponentBuilder":
        self._config["executable_path"] = path
        return self

    def with_source_path(self, path: str) -> "ExecutableComponentBuilder":
        self._config["source_path"] = path
        return self

    def with_build_tool(self, build_tool: BuildTool) -> "ExecutableComponentBuilder":
        self._config["build_tool"] = build_tool.value
        return self

    def with_build_tool_custom(self, command: str, args: List[str]) -> "ExecutableComponentBuilder":
        self._config["build_tool"] = BuildTool.custom(command, args)
        return self

    def with_env_var(self, key: str, value: str) -> "ExecutableComponentBuilder":
        self._config["env_vars"][key] = value
        return self

    def with_runtime_arg(self, name: str, value: str) -> "ExecutableComponentBuilder":
        self._config["runtime_args"].append({"name": name, "value": value})
        return self

    def with_readiness_check(
        self,
        check: Union[HttpReadinessCheck, TcpReadinessCheck],
        target: str,
        timeout_ms: int = 10_000,
    ) -> "ExecutableComponentBuilder":
        self._readiness_checks.append((check, target, timeout_ms))
        return self

    def with_child_components(self, children: List[Any]) -> "ExecutableComponentBuilder":
        self._children.extend(children)
        return self

    def build(self) -> "ExecutableComponent":
        return ExecutableComponent(
            dict(self._config),
            readiness_checks=list(self._readiness_checks),
            children=list(self._children),
        )

    def _for_ffi(self) -> Dict[str, Any]:
        d = dict(self._config)
        children = children_for_ffi(self._children)
        if children:
            d["children"] = children
        return d


class ExecutableComponent:
    def __init__(
        self,
        config: Dict[str, Any],
        readiness_checks: Optional[List[ReadinessCheckEntry]] = None,
        children: Optional[List[Any]] = None,
    ):
        self._config = config
        self._readiness_checks = readiness_checks or []
        self._children = children or []

    def _for_ffi(self) -> Dict[str, Any]:
        d = dict(self._config)
        rc = readiness_checks_for_ffi(self._readiness_checks)
        if rc:
            d["readiness_checks"] = rc
        children = children_for_ffi(self._children)
        if children:
            d["children"] = children
        return d
