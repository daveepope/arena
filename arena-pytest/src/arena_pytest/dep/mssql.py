from __future__ import annotations

import json
from typing import Any, Dict, List, Optional, TYPE_CHECKING

from arena_pytest.support._identifier import build as _build_identifier

if TYPE_CHECKING:
    from arena_pytest.arena import OpenArena


class MssqlDependencyBuilder:
    def __init__(self, name: str = ""):
        self._config: Dict[str, Any] = {
            "type": "mssql",
            "identifier": _build_identifier("arena-mssql", name),
        }

    def with_image_name(self, image_name: str) -> "MssqlDependencyBuilder":
        self._config["image_name"] = image_name
        return self

    def with_image(self, image: str) -> "MssqlDependencyBuilder":
        self._config["image"] = image
        return self

    def with_port(self, port: int) -> "MssqlDependencyBuilder":
        self._config["port"] = port
        return self

    def with_database_name(self, name: str) -> "MssqlDependencyBuilder":
        self._config["database_name"] = name
        return self

    def with_database_username(self, username: str) -> "MssqlDependencyBuilder":
        self._config["database_username"] = username
        return self

    def with_database_password(self, password: str) -> "MssqlDependencyBuilder":
        self._config["database_password"] = password
        return self

    def with_container_name(self, name: str) -> "MssqlDependencyBuilder":
        self._config["container_name"] = name
        return self

    def with_startup_sql_scripts(self, scripts: List[str]) -> "MssqlDependencyBuilder":
        self._config["startup_sql_scripts"] = scripts
        return self

    def build(self) -> "MssqlDependency":
        return MssqlDependency(dict(self._config))

    def _for_ffi(self) -> Dict[str, Any]:
        return dict(self._config)


class MssqlDependency:
    def __init__(self, config: Dict[str, Any]):
        self._config = config

    @property
    def identifier(self) -> str:
        return self._config["identifier"]

    def _for_ffi(self) -> Dict[str, Any]:
        return self._config


class ManagedMssqlPlaybook:
    def __init__(self, identifier: str, dependency_identifier: str):
        self._identifier = identifier
        self._dependency_identifier = dependency_identifier

    @property
    def identifier(self) -> str:
        return self._identifier

    @property
    def dependency_identifier(self) -> str:
        return self._dependency_identifier

    def _for_ffi(self) -> Dict[str, Any]:
        return {
            "identifier": self._identifier,
            "kind": "mssql",
            "dependency_identifier": self._dependency_identifier,
        }

    def run(self, arena: "OpenArena") -> "ActiveMssqlPlaybook":
        return ActiveMssqlPlaybook(
            arena=arena,
            dependency_identifier=self._dependency_identifier,
        )


class ActiveMssqlPlaybook:

    def __init__(self, arena: "OpenArena", dependency_identifier: str):
        self._arena = arena
        self._dependency_identifier = dependency_identifier
        self._handle: Optional[int] = None

    def __enter__(self) -> "ActiveMssqlPlaybook":
        from arena_pytest.ffi._ffi import mssql_playbook_begin

        spec = json.dumps({"dependency_identifier": self._dependency_identifier})
        self._handle = mssql_playbook_begin(
            self._arena._ffi, self._arena._handle, spec
        )
        return self

    def __exit__(self, exc_type, exc, tb) -> None:
        from arena_pytest.ffi._ffi import ArenaBindingError, mssql_playbook_finish

        handle = self._handle
        self._handle = None
        if not handle:
            return
        try:
            mssql_playbook_finish(self._arena._ffi, handle)
        except ArenaBindingError as e:
            if exc_type is not None:
                return
            raise AssertionError(str(e)) from None

    def finish(self) -> None:
        if self._handle:
            from arena_pytest.ffi._ffi import mssql_playbook_finish

            mssql_playbook_finish(self._arena._ffi, self._handle)
            self._handle = None

    def verify(self, query: str, expected_value: int) -> None:
        from arena_pytest.ffi._ffi import mssql_playbook_verify

        if not self._handle:
            raise RuntimeError(
                "ActiveMssqlPlaybook.verify requires begun playbook scope "
                "(enter `with` or __enter__ first)"
            )
        spec = json.dumps({
            "dependency_identifier": self._dependency_identifier,
            "query": query,
            "expected_value": int(expected_value),
        })
        mssql_playbook_verify(self._arena._ffi, self._handle, spec)


class ManagedMssqlPlaybookBuilder:
    def __init__(self, identifier: str, dependency_identifier: str):
        self._identifier = identifier
        self._dependency_identifier = dependency_identifier

    def build(self) -> ManagedMssqlPlaybook:
        return ManagedMssqlPlaybook(
            self._identifier, self._dependency_identifier
        )
