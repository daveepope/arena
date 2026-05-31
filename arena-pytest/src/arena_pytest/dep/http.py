from __future__ import annotations

import json
from typing import Any, Dict, List, Optional, TYPE_CHECKING

from arena_pytest.ffi._ffi import http_playbook_open, match_playbook_run
from arena_pytest.playbook import ActiveHttpPlaybook, Playbook
from arena_pytest.support._identifier import build as _build_identifier

if TYPE_CHECKING:
    from arena_pytest.arena import OpenArena


class HttpMappingExpect:
    @staticmethod
    def called(count: int) -> Dict[str, Any]:
        return {"kind": "exactly", "count": int(count)}

    @staticmethod
    def called_at_least(count: int) -> Dict[str, Any]:
        return {"kind": "at_least", "count": int(count)}

    @staticmethod
    def never_called() -> Dict[str, Any]:
        return {"kind": "never"}


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


class ManagedHttpPlaybook(Playbook):
    def __init__(
        self,
        *,
        identifier: str,
        dependency_identifier: str,
        mappings: List[Dict[str, Any]],
    ):
        if not mappings:
            raise ValueError(
                "ManagedHttpPlaybook requires at least one mapping"
            )
        self._identifier = identifier
        self._dependency_identifier = dependency_identifier
        self._mappings = [dict(m) for m in mappings]

    def identifier(self) -> str:
        return self._identifier

    @property
    def dependency_identifier(self) -> str:
        return self._dependency_identifier

    def _for_ffi(self) -> Dict[str, Any]:
        return {
            "identifier": self._identifier,
            "kind": "http",
            "dependency_identifier": self._dependency_identifier,
            "mappings": [dict(m) for m in self._mappings],
        }

    def run(self, arena: "OpenArena") -> ActiveHttpPlaybook:
        handle = match_playbook_run(arena._ffi, arena._handle, self._identifier)
        return ActiveHttpPlaybook(arena._ffi, handle, self._dependency_identifier)


class ActiveHttpPlaybookBuilder:
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
    ) -> "ActiveHttpPlaybookBuilder":
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

    def build(self, arena: "OpenArena") -> "_ScopedActiveHttpPlaybook":
        if not self._mappings:
            raise ValueError(
                "ActiveHttpPlaybookBuilder requires at least one mapping"
            )
        return _ScopedActiveHttpPlaybook(
            arena,
            self._dependency_identifier,
            [dict(m) for m in self._mappings],
        )


HttpPlaybookBuilder = ActiveHttpPlaybookBuilder


class _ScopedActiveHttpPlaybook:
    def __init__(
        self,
        arena: "OpenArena",
        dependency_identifier: str,
        mappings: List[Dict[str, Any]],
    ):
        self._arena = arena
        self._dependency_identifier = dependency_identifier
        self._mappings = mappings
        self._active: Optional[ActiveHttpPlaybook] = None

    def __enter__(self) -> ActiveHttpPlaybook:
        spec = json.dumps({
            "dependency_identifier": self._dependency_identifier,
            "mappings": self._mappings,
        })
        handle = http_playbook_open(self._arena._ffi, self._arena._handle, spec)
        self._active = ActiveHttpPlaybook(
            self._arena._ffi,
            handle,
            self._dependency_identifier,
        )
        return self._active

    def __exit__(self, exc_type, exc, tb) -> None:
        if self._active is None:
            return
        self._active.__exit__(exc_type, exc, tb)
        self._active = None
