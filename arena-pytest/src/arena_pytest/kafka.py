from enum import Enum
from typing import Any, Dict

from arena_pytest._identifier import build as _build_identifier

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

    def build(self) -> "KafkaDependency":
        return KafkaDependency(dict(self._config))

    def _for_ffi(self) -> Dict[str, Any]:
        return dict(self._config)


class KafkaDependency:
    def __init__(self, config: Dict[str, Any]):
        self._config = config

    @property
    def identifier(self) -> str:
        return self._config["identifier"]

    def _for_ffi(self) -> Dict[str, Any]:
        return self._config
