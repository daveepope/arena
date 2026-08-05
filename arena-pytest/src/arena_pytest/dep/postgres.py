from __future__ import annotations

from typing import Any, Dict, List, TYPE_CHECKING

from arena_pytest.ffi._ffi import match_playbook_run
from arena_pytest.playbook import ActivePostgresPlaybook, Playbook
from arena_pytest.support._identifier import build as _build_identifier

if TYPE_CHECKING:
    from arena_pytest.arena import OpenArena


class PostgresDependencyBuilder:
    def __init__(self, name: str = ""):
        self._config: Dict[str, Any] = {
            "type": "postgres",
            "identifier": _build_identifier("arena-postgres", name),
        }

    def with_image_name(self, image_name: str) -> "PostgresDependencyBuilder":
        self._config["image_name"] = image_name
        return self

    def with_image(self, image: str) -> "PostgresDependencyBuilder":
        self._config["image"] = image
        return self

    def with_port(self, port: int) -> "PostgresDependencyBuilder":
        self._config["port"] = port
        return self

    def with_database_name(self, name: str) -> "PostgresDependencyBuilder":
        self._config["database_name"] = name
        return self

    def with_database_username(self, username: str) -> "PostgresDependencyBuilder":
        self._config["database_username"] = username
        return self

    def with_database_password(self, password: str) -> "PostgresDependencyBuilder":
        self._config["database_password"] = password
        return self

    def with_container_name(self, name: str) -> "PostgresDependencyBuilder":
        self._config["container_name"] = name
        return self

    def with_startup_sql_scripts(self, scripts: List[str]) -> "PostgresDependencyBuilder":
        self._config["startup_sql_scripts"] = scripts
        return self

    def build(self) -> "PostgresDependency":
        return PostgresDependency(dict(self._config))

    def _for_ffi(self) -> Dict[str, Any]:
        return dict(self._config)


class PostgresDependency:
    def __init__(self, config: Dict[str, Any]):
        self._config = config

    @property
    def identifier(self) -> str:
        return self._config["identifier"]

    def _for_ffi(self) -> Dict[str, Any]:
        return self._config


class ManagedPostgresPlaybook(Playbook):
    def __init__(
        self,
        *,
        identifier: str,
        dependency_identifier: str,
    ):
        self._identifier = identifier
        self._dependency_identifier = dependency_identifier

    def identifier(self) -> str:
        return self._identifier

    @property
    def dependency_identifier(self) -> str:
        return self._dependency_identifier

    def _for_ffi(self) -> Dict[str, Any]:
        return {
            "identifier": self._identifier,
            "kind": "postgres",
            "dependency_identifier": self._dependency_identifier,
        }

    def run(self, arena: "OpenArena") -> ActivePostgresPlaybook:
        handle = match_playbook_run(arena._ffi, arena._handle, self._identifier)
        return ActivePostgresPlaybook(arena._ffi, handle, self._dependency_identifier)
