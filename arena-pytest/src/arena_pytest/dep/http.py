from __future__ import annotations

import json
from typing import Any, Callable, Dict, List, Optional, TYPE_CHECKING, Union

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


class HttpHeaderPattern:
    @staticmethod
    def equal_to(value: str) -> Dict[str, str]:
        return {"equal_to": value}

    @staticmethod
    def matching(regex: str) -> Dict[str, str]:
        return {"matches": regex}


class HttpResponse:
    def __init__(self, status: int = 200):
        self._data: Dict[str, Any] = {"status": int(status)}

    def with_status(self, status: int) -> "HttpResponse":
        self._data["status"] = int(status)
        return self

    def with_json_body(self, body: Any) -> "HttpResponse":
        self._data["json_body"] = body
        return self

    def with_header(self, name: str, value: str) -> "HttpResponse":
        headers = self._data.setdefault("headers", {})
        headers[name] = value
        return self

    def with_fixed_delay_ms(self, ms: int) -> "HttpResponse":
        self._data["fixed_delay_ms"] = int(ms)
        return self

    def with_uniform_random_delay_ms(self, lower: int, upper: int) -> "HttpResponse":
        self._data["delay_distribution"] = {
            "type": "uniform",
            "lower": int(lower),
            "upper": int(upper),
        }
        return self

    def _for_spec(self) -> Dict[str, Any]:
        return dict(self._data)


def ok() -> HttpResponse:
    return HttpResponse(200)


def ok_json(body: Any) -> HttpResponse:
    return HttpResponse(200).with_json_body(body)


def status(code: int) -> HttpResponse:
    return HttpResponse(code)


def server_error() -> HttpResponse:
    return HttpResponse(500)


def created() -> HttpResponse:
    return HttpResponse(201)


def no_content() -> HttpResponse:
    return HttpResponse(204)


def _coerce_response(
    response: Optional[Union[HttpResponse, Dict[str, Any]]] = None,
    *,
    status: int = 200,
    json_body: Optional[Any] = None,
) -> HttpResponse:
    if response is None:
        out = HttpResponse(status)
        if json_body is not None:
            out = out.with_json_body(json_body)
        return out
    if isinstance(response, HttpResponse):
        return response
    out = HttpResponse(int(response.get("status", 200)))
    if "json_body" in response:
        out = out.with_json_body(response["json_body"])
    if "headers" in response:
        for name, value in response["headers"].items():
            out = out.with_header(name, value)
    if "fixed_delay_ms" in response:
        out = out.with_fixed_delay_ms(int(response["fixed_delay_ms"]))
    if "delay_distribution" in response:
        dist = response["delay_distribution"]
        out = out.with_uniform_random_delay_ms(
            int(dist["lower"]),
            int(dist["upper"]),
        )
    return out


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

    def playbook(self) -> "HttpPlaybookBuilder":
        return HttpPlaybookBuilder(self.identifier)

    def _for_ffi(self) -> Dict[str, Any]:
        return self._config


class HttpPlaybookBuilder:
    def __init__(self, dependency_identifier: str):
        self._dependency_identifier = dependency_identifier
        self._mappings: List[Dict[str, Any]] = []

    def get(self, url_path: str) -> "HttpMappingBuilder":
        return HttpMappingBuilder(self, "GET", url_path)

    def post(self, url_path: str) -> "HttpMappingBuilder":
        return HttpMappingBuilder(self, "POST", url_path)

    def put(self, url_path: str) -> "HttpMappingBuilder":
        return HttpMappingBuilder(self, "PUT", url_path)

    def delete(self, url_path: str) -> "HttpMappingBuilder":
        return HttpMappingBuilder(self, "DELETE", url_path)

    def _append_mapping(self, mapping: Dict[str, Any]) -> None:
        self._mappings.append(dict(mapping))

    def mappings_for_ffi(self) -> List[Dict[str, Any]]:
        if not self._mappings:
            raise ValueError("HttpPlaybookBuilder requires at least one mapping")
        return [dict(m) for m in self._mappings]

    def into_playbook(self) -> "HttpPlaybookBuilder":
        return self

    def open(self, arena: "OpenArena") -> "_ScopedActiveHttpPlaybook":
        return _ScopedActiveHttpPlaybook(
            arena,
            self._dependency_identifier,
            self.mappings_for_ffi(),
        )

    def build(self, arena: "OpenArena") -> "_ScopedActiveHttpPlaybook":
        return self.open(arena)


class HttpMappingBuilder:
    def __init__(
        self,
        playbook: HttpPlaybookBuilder,
        method: str,
        url_path: str,
    ):
        self._playbook = playbook
        self._spec: Dict[str, Any] = {
            "method": method.upper(),
            "url_path": url_path,
        }

    def with_header(
        self,
        name: str,
        pattern: Dict[str, str],
    ) -> "HttpMappingBuilder":
        headers = self._spec.setdefault("headers", {})
        headers[name] = dict(pattern)
        return self

    def with_request_body(self, body: Any) -> "HttpMappingBuilder":
        patterns = self._spec.setdefault("body_patterns", [])
        patterns.append({"equal_to_json": json.dumps(body)})
        return self

    def with_request_body_containing(self, substring: str) -> "HttpMappingBuilder":
        patterns = self._spec.setdefault("body_patterns", [])
        patterns.append({"contains": substring})
        return self

    def with_priority(self, priority: int) -> "HttpMappingBuilder":
        self._spec["priority"] = int(priority)
        return self

    def in_scenario(self, name: str) -> "HttpMappingBuilder":
        self._spec["scenario_name"] = name
        return self

    def when_state_is(self, state: str) -> "HttpMappingBuilder":
        self._spec["when_state_is"] = state
        return self

    def will_set_state_to(self, state: str) -> "HttpMappingBuilder":
        self._spec["will_set_state_to"] = state
        return self

    def will_return(
        self,
        response: Optional[Union[HttpResponse, Dict[str, Any]]] = None,
        *,
        status: int = 200,
        json_body: Optional[Any] = None,
    ) -> "HttpSequenceBuilder":
        return HttpSequenceBuilder(
            self._playbook,
            dict(self._spec),
            [_coerce_response(response, status=status, json_body=json_body)],
        )

    def will_return_in_sequence(
        self,
        responses: List[Union[HttpResponse, Dict[str, Any]]],
    ) -> HttpPlaybookBuilder:
        spec = dict(self._spec)
        spec["responses"] = [_coerce_response(r)._for_spec() for r in responses]
        self._playbook._append_mapping(spec)
        return self._playbook


class HttpSequenceBuilder:
    def __init__(
        self,
        playbook: HttpPlaybookBuilder,
        mapping_spec: Dict[str, Any],
        responses: List[HttpResponse],
    ):
        self._playbook = playbook
        self._mapping_spec = mapping_spec
        self._responses = list(responses)
        self._expect: Optional[Dict[str, Any]] = None

    def then_return(
        self,
        response: Optional[Union[HttpResponse, Dict[str, Any]]] = None,
        *,
        status: int = 200,
        json_body: Optional[Any] = None,
    ) -> "HttpSequenceBuilder":
        self._responses.append(
            _coerce_response(response, status=status, json_body=json_body)
        )
        return self

    def expect_called(self, count: int) -> "HttpSequenceBuilder":
        self._expect = HttpMappingExpect.called(count)
        return self

    def expect_called_at_least(self, count: int) -> "HttpSequenceBuilder":
        self._expect = HttpMappingExpect.called_at_least(count)
        return self

    def expect_never_called(self) -> "HttpSequenceBuilder":
        self._expect = HttpMappingExpect.never_called()
        return self

    def _finalize_spec(self) -> Dict[str, Any]:
        spec = dict(self._mapping_spec)
        if len(self._responses) == 1:
            spec["response"] = self._responses[0]._for_spec()
        else:
            spec["responses"] = [r._for_spec() for r in self._responses]
        if self._expect is not None:
            spec["expect"] = dict(self._expect)
        return spec

    def into_playbook(self) -> HttpPlaybookBuilder:
        self._playbook._append_mapping(self._finalize_spec())
        return self._playbook

    def get(self, url_path: str) -> HttpMappingBuilder:
        return self.into_playbook().get(url_path)

    def post(self, url_path: str) -> HttpMappingBuilder:
        return self.into_playbook().post(url_path)

    def put(self, url_path: str) -> HttpMappingBuilder:
        return self.into_playbook().put(url_path)

    def delete(self, url_path: str) -> HttpMappingBuilder:
        return self.into_playbook().delete(url_path)

    def open(self, arena: "OpenArena") -> "_ScopedActiveHttpPlaybook":
        return self.into_playbook().open(arena)

    def build(self, arena: "OpenArena") -> "_ScopedActiveHttpPlaybook":
        return self.open(arena)


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

    @classmethod
    def from_builder(
        cls,
        identifier: str,
        dependency_identifier: str,
        build: Callable[[HttpPlaybookBuilder], HttpPlaybookBuilder],
    ) -> "ManagedHttpPlaybook":
        built = build(HttpPlaybookBuilder(dependency_identifier))
        if isinstance(built, HttpSequenceBuilder):
            built = built.into_playbook()
        return cls(
            identifier=identifier,
            dependency_identifier=dependency_identifier,
            mappings=built.mappings_for_ffi(),
        )

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
        self._builder = HttpPlaybookBuilder(dependency_identifier)

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

        mapping = HttpMappingBuilder(self._builder, method.upper(), url_path)
        if priority is not None:
            mapping = mapping.with_priority(priority)
        chain = mapping.will_return(status=status, json_body=json_body)
        if expect_called is not None:
            chain = chain.expect_called(expect_called)
        elif expect_called_at_least is not None:
            chain = chain.expect_called_at_least(expect_called_at_least)
        elif expect_never_called:
            chain = chain.expect_never_called()

        self._builder = chain.into_playbook()
        return self

    def build(self, arena: "OpenArena") -> "_ScopedActiveHttpPlaybook":
        return self._builder.open(arena)


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
