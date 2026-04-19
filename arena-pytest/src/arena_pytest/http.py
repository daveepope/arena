from __future__ import annotations

import json
from typing import Any, Dict, List, Optional, TYPE_CHECKING

if TYPE_CHECKING:
    from arena_pytest._ffi import ArenaFfi


from arena_pytest._identifier import build as _build_identifier


class HttpDependencyBuilder:
    def __init__(self, name: str = ""):
        self._config: Dict[str, Any] = {
            "type": "http",
            "identifier": _build_identifier("arena-http", name),
        }

    def with_port(self, port: int) -> "HttpDependencyBuilder":
        self._config["port"] = port
        return self

    def with_container_name(self, name: str) -> "HttpDependencyBuilder":
        self._config["container_name"] = name
        return self

    def with_image_name(self, image_name: str) -> "HttpDependencyBuilder":
        self._config["image_name"] = image_name
        return self

    def with_image_tag(self, image_tag: str) -> "HttpDependencyBuilder":
        self._config["image_tag"] = image_tag
        return self

    def build(self) -> "HttpDependency":
        return HttpDependency(dict(self._config))

    def _for_ffi(self) -> Dict[str, Any]:
        return dict(self._config)


class HttpDependency:
    def __init__(self, config: Dict[str, Any]):
        self._config = config

    @property
    def identifier(self) -> str:
        return self._config["identifier"]

    def _for_ffi(self) -> Dict[str, Any]:
        return self._config


class HttpOnDependencyStartupBuilder:
    def __init__(self, dependency_identifier: str):
        self._dependency_identifier = dependency_identifier
        self._mappings: List[Dict[str, Any]] = []

    def with_mapping(
        self,
        method: str,
        url_path: str,
        status: int = 200,
        json_body: Optional[Any] = None,
    ) -> "HttpOnDependencyStartupBuilder":
        mapping: Dict[str, Any] = {
            "method": method.upper(),
            "url_path": url_path,
            "status": status,
        }
        if json_body is not None:
            mapping["json_body"] = json_body
        self._mappings.append(mapping)
        return self

    def build(self) -> "HttpOnDependencyStartup":
        return HttpOnDependencyStartup(self._dependency_identifier, list(self._mappings))


class HttpOnDependencyStartup:
    def __init__(self, dependency_identifier: str, mappings: List[Dict[str, Any]]):
        self._dependency_identifier = dependency_identifier
        self._mappings = mappings

    def _for_ffi(self) -> Dict[str, Any]:
        return {
            "kind": "http",
            "dependency_identifier": self._dependency_identifier,
            "mappings": list(self._mappings),
        }


class HttpPlaybookBuilder:
    def __init__(self, dependency_identifier: str):
        self._dependency_identifier = dependency_identifier
        self._mappings: List[Dict[str, Any]] = []

    def with_mapping(
        self,
        method: str,
        url_path: str,
        status: int = 200,
        json_body: Optional[Any] = None,
        priority: Optional[int] = None,
        expect_called: Optional[int] = None,
        expect_called_at_least: Optional[int] = None,
        expect_never_called: bool = False,
    ) -> "HttpPlaybookBuilder":
        response: Dict[str, Any] = {"status": status}
        if json_body is not None:
            response["json_body"] = json_body
        mapping: Dict[str, Any] = {
            "method": method.upper(),
            "url_path": url_path,
            "response": response,
        }
        if priority is not None:
            mapping["priority"] = priority

        expects_set = [
            expect_called is not None,
            expect_called_at_least is not None,
            expect_never_called,
        ]
        if sum(1 for v in expects_set if v) > 1:
            raise ValueError(
                "with_mapping accepts at most one of: expect_called, "
                "expect_called_at_least, expect_never_called"
            )
        if expect_called is not None:
            mapping["expect"] = {"kind": "exactly", "count": int(expect_called)}
        elif expect_called_at_least is not None:
            mapping["expect"] = {
                "kind": "at_least",
                "count": int(expect_called_at_least),
            }
        elif expect_never_called:
            mapping["expect"] = {"kind": "never"}

        self._mappings.append(mapping)
        return self

    def build(self, arena: "OpenArena") -> "HttpPlaybook":
        if not self._mappings:
            raise ValueError("HttpPlaybookBuilder requires at least one mapping")
        return HttpPlaybook(
            arena=arena,
            dependency_identifier=self._dependency_identifier,
            mappings=list(self._mappings),
        )


class HttpPlaybook:
    def __init__(
        self,
        arena: "OpenArena",
        dependency_identifier: str,
        mappings: List[Dict[str, Any]],
    ):
        self._arena = arena
        self._dependency_identifier = dependency_identifier
        self._mappings = mappings
        self._handle: Optional[int] = None

    def __enter__(self) -> "HttpPlaybook":
        from arena_pytest._ffi import http_playbook_open

        spec = json.dumps({
            "dependency_identifier": self._dependency_identifier,
            "mappings": self._mappings,
        })
        self._handle = http_playbook_open(self._arena._ffi, self._arena._handle, spec)
        return self

    def __exit__(self, exc_type, exc, tb) -> None:
        from arena_pytest._ffi import ArenaFfiError, http_playbook_close

        handle = self._handle
        self._handle = None
        if not handle:
            return

        try:
            http_playbook_close(self._arena._ffi, handle)
        except ArenaFfiError as e:
            if exc_type is not None:
                return
            raise AssertionError(str(e)) from None

    def close(self) -> None:
        if self._handle:
            from arena_pytest._ffi import http_playbook_close

            http_playbook_close(self._arena._ffi, self._handle)
            self._handle = None

    def verify(self, method: str, url_path: str, expected_count: int) -> None:
        from arena_pytest._ffi import http_playbook_verify

        if not self._handle:
            raise RuntimeError("HttpPlaybook.verify called outside of context")
        spec = json.dumps({
            "dependency_identifier": self._dependency_identifier,
            "method": method.upper(),
            "url_path": url_path,
            "expected_count": expected_count,
        })
        http_playbook_verify(self._arena._ffi, self._handle, spec)


if TYPE_CHECKING:
    from arena_pytest.arena import OpenArena
