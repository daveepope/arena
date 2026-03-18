from typing import Any, Dict, List


class PostgresDependencyBuilder:
    def __init__(self, identifier: str):
        self._config: Dict[str, Any] = {"type": "postgres", "identifier": identifier}

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

    def _for_ffi(self) -> Dict[str, Any]:
        return self._config
