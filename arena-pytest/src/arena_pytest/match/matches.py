from typing import Any, Dict, List, Optional


class Match:
    def __init__(
        self,
        name: str,
        dependencies: List[Any],
        components: List[Any],
        network: Optional[str] = None,
        playbooks: Optional[List[Any]] = None,
    ):
        self._name = name
        self._dependencies = dependencies
        self._components = components
        self._network = network
        self._playbooks = playbooks or []

    def _for_ffi(self) -> Dict[str, Any]:
        out: Dict[str, Any] = {
            "match_name": self._name,
            "dependencies": [
                d._for_ffi() if hasattr(d, "_for_ffi") else d for d in self._dependencies
            ],
            "components": [
                c._for_ffi() if hasattr(c, "_for_ffi") else c for c in self._components
            ],
        }
        if self._network:
            out["network"] = self._network
        if self._playbooks:
            out["playbooks"] = [
                p._for_ffi() if hasattr(p, "_for_ffi") else p for p in self._playbooks
            ]
        return out


class MatchBuilder:
    def __init__(self, name: str):
        self._name = name
        self._network: Optional[str] = None
        self._dependencies: List[Any] = []
        self._components: List[Any] = []
        self._playbooks: List[Any] = []

    def with_network(self, network: str) -> "MatchBuilder":
        self._network = network
        return self

    def add_dependency(self, dep: Any) -> "MatchBuilder":
        self._dependencies.append(dep)
        return self

    def add_component(self, comp: Any) -> "MatchBuilder":
        self._components.append(comp)
        return self

    def register_playbook(
        self,
        playbook: Any,
        exec_on_dependency_start: bool = True,
    ) -> "MatchBuilder":
        self._playbooks.append(
            _RegisteredPlaybook(playbook, exec_on_dependency_start)
        )
        return self

    def build(self) -> Match:
        return Match(
            self._name,
            self._dependencies,
            self._components,
            self._network,
            self._playbooks,
        )


class _RegisteredPlaybook:
    def __init__(self, playbook: Any, exec_on_dependency_start: bool):
        self._playbook = playbook
        self._exec_on_dependency_start = exec_on_dependency_start

    def _for_ffi(self) -> Dict[str, Any]:
        if not hasattr(self._playbook, "_for_ffi"):
            raise TypeError(
                "register_playbook expects an object with a _for_ffi() method"
            )
        cfg = dict(self._playbook._for_ffi())
        cfg["exec_on_dependency_start"] = self._exec_on_dependency_start
        return cfg
