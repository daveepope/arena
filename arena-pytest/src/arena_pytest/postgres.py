from typing import Any, Dict, List

from arena_pytest._identifier import build as _build_identifier


class PostgresDependencyBuilder:
    def __init__(self, name: str = ""):
        self._config: Dict[str, Any] = {
            "type": "postgres",
            "identifier": _build_identifier("arena-postgres", name),
        }

    def with_image_name(self, image_name: str) -> "PostgresDependencyBuilder":
        self._config["image_name"] = image_name
        return self

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
