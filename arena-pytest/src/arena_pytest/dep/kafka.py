from enum import Enum
from typing import Any, Dict, List, Optional

from arena_pytest.ffi._ffi_children import children_for_ffi
from arena_pytest.support._identifier import build as _build_identifier

KAFKA_INTERNAL_DOCKER_PORT = 29092


class KafkaFlavor(Enum):
    APACHE_NATIVE = "apache_native"
    CONFLUENT = "confluent"


class KafkaDependencyBuilder:
    def __init__(self, name: str = ""):
        self._config: Dict[str, Any] = {
            "type": "kafka",
            "identifier": _build_identifier("arena-kafka", name),
            "topics": [],
        }
        self._children: List[Any] = []

    def with_image_name(self, image_name: str) -> "KafkaDependencyBuilder":
        self._config["image_name"] = image_name
        return self

    def with_topic(self, topic: str) -> "KafkaDependencyBuilder":
        self._config["topics"].append(topic)
        return self

    def with_flavor(self, flavor: KafkaFlavor) -> "KafkaDependencyBuilder":
        self._config["flavor"] = flavor.value
        return self

    def with_port(self, port: int) -> "KafkaDependencyBuilder":
        self._config["port"] = port
        return self

    def with_container_name(self, name: str) -> "KafkaDependencyBuilder":
        self._config["container_name"] = name
        return self

    def with_child_dependencies(self, children: List[Any]) -> "KafkaDependencyBuilder":
        self._children.extend(children)
        return self

    def build(self) -> "KafkaDependency":
        return KafkaDependency(dict(self._config), children=list(self._children))

    def _for_ffi(self) -> Dict[str, Any]:
        d = dict(self._config)
        children = children_for_ffi(self._children)
        if children:
            d["children"] = children
        return d


class KafkaDependency:
    def __init__(self, config: Dict[str, Any], children: Optional[List[Any]] = None):
        self._config = config
        self._children = children or []

    @property
    def identifier(self) -> str:
        return self._config["identifier"]

    def _for_ffi(self) -> Dict[str, Any]:
        d = dict(self._config)
        children = children_for_ffi(self._children)
        if children:
            d["children"] = children
        return d
