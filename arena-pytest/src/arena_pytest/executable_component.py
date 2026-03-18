from enum import Enum
from typing import Any, Dict, List, Optional


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
    def __init__(self, identifier: str):
        self._config: Dict[str, Any] = {
            "type": "exec",
            "identifier": identifier,
            "env_vars": {},
            "runtime_args": {},
        }

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

    def with_runtime_arg(self, key: str, value: str) -> "ExecutableComponentBuilder":
        self._config["runtime_args"][key] = value
        return self

    def with_readiness_check_url(self, url: str) -> "ExecutableComponentBuilder":
        self._config["readiness_check_url"] = url
        return self

    def build(self) -> "ExecutableComponent":
        return ExecutableComponent(dict(self._config))

    def _for_ffi(self) -> Dict[str, Any]:
        return dict(self._config)


class ExecutableComponent:
    def __init__(self, config: Dict[str, Any]):
        self._config = config

    def _for_ffi(self) -> Dict[str, Any]:
        return self._config
