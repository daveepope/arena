from __future__ import annotations

import os
from dataclasses import dataclass, field
from typing import Any, Dict, List, Optional, Tuple, TYPE_CHECKING, Union

from arena_pytest.ffi._ffi import match_playbook_run
from arena_pytest.ffi._ffi_children import children_for_ffi
from arena_pytest.playbook import ActiveLocalstackPlaybook, ManagedPlaybook
from arena_pytest.support._identifier import build as _build_identifier

if TYPE_CHECKING:
    from arena_pytest.arena import OpenArena


LOCALSTACK_INTERNAL_DOCKER_PORT = 4566
LOCALSTACK_DEFAULT_ACCOUNT_ID = "000000000000"
LOCALSTACK_DEFAULT_REGION = "us-east-1"


@dataclass
class QueueSpec:
    name: str
    fifo: bool = False


@dataclass
class LambdaSpec:
    name: str
    runtime: str
    handler: str
    source_dir: str
    environment: List[Tuple[str, str]] = field(default_factory=list)


@dataclass
class EventBusSpec:
    name: str


@dataclass
class SqsQueueTarget:
    queue_name: str


@dataclass
class LambdaTarget:
    function_name: str


EventTargetKind = Union[SqsQueueTarget, LambdaTarget]


@dataclass
class EventRuleTarget:
    target_id: str
    kind: EventTargetKind


@dataclass
class EventRuleSpec:
    name: str
    event_pattern: str
    targets: List[EventRuleTarget]
    event_bus: Optional[str] = None


def _resolve_source_dir(source_dir: str) -> str:
    path = os.path.expanduser(source_dir)
    if not os.path.isabs(path):
        path = os.path.abspath(path)
    return path


def _target_kind_for_ffi(kind: EventTargetKind) -> Dict[str, Any]:
    if isinstance(kind, SqsQueueTarget):
        return {"kind": "sqs_queue", "queue_name": kind.queue_name}
    if isinstance(kind, LambdaTarget):
        return {"kind": "lambda", "function_name": kind.function_name}
    raise TypeError(f"unsupported event target kind: {type(kind).__name__}")


class LocalstackDependencyBuilder:
    def __init__(self, name: str = ""):
        self._config: Dict[str, Any] = {
            "type": "localstack",
            "identifier": _build_identifier("arena-localstack", name),
            "services": [],
            "queues": [],
            "lambdas": [],
            "event_buses": [],
            "event_rules": [],
        }
        self._children: List[Any] = []

    def with_port(self, port: int) -> "LocalstackDependencyBuilder":
        self._config["port"] = int(port)
        return self

    def with_image_name(self, image_name: str) -> "LocalstackDependencyBuilder":
        self._config["image_name"] = image_name
        return self

    def with_image_tag(self, image_tag: str) -> "LocalstackDependencyBuilder":
        self._config["image_tag"] = image_tag
        return self

    def with_container_name(self, name: str) -> "LocalstackDependencyBuilder":
        self._config["container_name"] = name
        return self

    def with_service(self, service: str) -> "LocalstackDependencyBuilder":
        self._config["services"].append(service)
        return self

    def with_services(self, services: List[str]) -> "LocalstackDependencyBuilder":
        self._config["services"].extend(services)
        return self

    def with_queue(self, name: str) -> "LocalstackDependencyBuilder":
        self._config["queues"].append({"name": name, "fifo": False})
        return self

    def with_fifo_queue(self, name: str) -> "LocalstackDependencyBuilder":
        self._config["queues"].append({"name": name, "fifo": True})
        return self

    def with_queue_spec(self, spec: QueueSpec) -> "LocalstackDependencyBuilder":
        self._config["queues"].append({"name": spec.name, "fifo": spec.fifo})
        return self

    def with_lambda(self, spec: LambdaSpec) -> "LocalstackDependencyBuilder":
        self._config["lambdas"].append({
            "name": spec.name,
            "runtime": spec.runtime,
            "handler": spec.handler,
            "source_dir": _resolve_source_dir(spec.source_dir),
            "environment": [list(pair) for pair in spec.environment],
        })
        return self

    def with_event_bus(self, name: str) -> "LocalstackDependencyBuilder":
        self._config["event_buses"].append({"name": name})
        return self

    def with_event_rule(self, spec: EventRuleSpec) -> "LocalstackDependencyBuilder":
        self._config["event_rules"].append({
            "name": spec.name,
            "event_bus": spec.event_bus,
            "event_pattern": spec.event_pattern,
            "targets": [
                {
                    "target_id": t.target_id,
                    **_target_kind_for_ffi(t.kind),
                }
                for t in spec.targets
            ],
        })
        return self

    def with_child_dependencies(self, children: List[Any]) -> "LocalstackDependencyBuilder":
        self._children.extend(children)
        return self

    def build(self) -> "LocalstackDependency":
        return LocalstackDependency(dict(self._config), children=list(self._children))

    def _for_ffi(self) -> Dict[str, Any]:
        d = dict(self._config)
        children = children_for_ffi(self._children)
        if children:
            d["children"] = children
        return d


class LocalstackDependency:
    def __init__(self, config: Dict[str, Any], children: Optional[List[Any]] = None):
        self._config = config
        self._children = children or []

    @property
    def identifier(self) -> str:
        return self._config["identifier"]

    @property
    def port(self) -> int:
        return int(self._config.get("port", LOCALSTACK_INTERNAL_DOCKER_PORT))

    def endpoint_url(self, host: str = "localhost") -> str:
        return f"http://{host}:{self.port}"

    def internal_endpoint_url(self, container_name: Optional[str] = None) -> str:
        name = container_name or self._config.get("container_name") or self.identifier
        return f"http://{name}:{LOCALSTACK_INTERNAL_DOCKER_PORT}"

    def queue_url(
        self,
        queue_name: str,
        host: str = "localhost",
        account_id: str = LOCALSTACK_DEFAULT_ACCOUNT_ID,
    ) -> str:
        return f"{self.endpoint_url(host)}/{account_id}/{queue_name}"

    def queue_arn(
        self,
        queue_name: str,
        region: str = LOCALSTACK_DEFAULT_REGION,
        account_id: str = LOCALSTACK_DEFAULT_ACCOUNT_ID,
    ) -> str:
        return f"arn:aws:sqs:{region}:{account_id}:{queue_name}"

    def lambda_arn(
        self,
        function_name: str,
        region: str = LOCALSTACK_DEFAULT_REGION,
        account_id: str = LOCALSTACK_DEFAULT_ACCOUNT_ID,
    ) -> str:
        return f"arn:aws:lambda:{region}:{account_id}:function:{function_name}"

    def _for_ffi(self) -> Dict[str, Any]:
        d = dict(self._config)
        children = children_for_ffi(self._children)
        if children:
            d["children"] = children
        return d


class ManagedLocalstackPlaybook(ManagedPlaybook):
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
            "kind": "localstack",
            "dependency_identifier": self._dependency_identifier,
        }

    def run(self, arena: "OpenArena") -> ActiveLocalstackPlaybook:
        handle = match_playbook_run(arena._ffi, arena._handle, self._identifier)
        return ActiveLocalstackPlaybook(arena._ffi, handle)
