import json
from typing import Any, Dict, List, Optional


class Encounter:
    def __init__(
        self,
        name: str,
        dependencies: List[Any],
        components: List[Any],
        network: Optional[str] = None,
    ):
        self._name = name
        self._dependencies = dependencies
        self._components = components
        self._network = network

    def _for_ffi(self) -> Dict[str, Any]:
        out: Dict[str, Any] = {
            "encounter_name": self._name,
            "dependencies": [
                d._for_ffi() if hasattr(d, "_for_ffi") else d for d in self._dependencies
            ],
            "components": [
                c._for_ffi() if hasattr(c, "_for_ffi") else c for c in self._components
            ],
        }
        if self._network:
            out["network"] = self._network
        return out


class EncounterBuilder:
    def __init__(self, name: str):
        self._name = name
        self._network: Optional[str] = None
        self._dependencies: List[Any] = []
        self._components: List[Any] = []

    def with_network(self, network: str) -> "EncounterBuilder":
        self._network = network
        return self

    def add_dependency(self, dep: Any) -> "EncounterBuilder":
        self._dependencies.append(dep)
        return self

    def add_component(self, comp: Any) -> "EncounterBuilder":
        self._components.append(comp)
        return self

    def build(self) -> Encounter:
        return Encounter(
            self._name,
            self._dependencies,
            self._components,
            self._network,
        )
