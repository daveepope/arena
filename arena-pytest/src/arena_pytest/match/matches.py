from __future__ import annotations

from typing import Any, Dict, List, Optional, Tuple, Type

from arena_pytest.playbook import Playbook


class Match:
    def __init__(
        self,
        name: str,
        dependencies: List[Any],
        components: List[Any],
        network: Optional[str],
        playbooks: Dict[Type[Playbook], Tuple[Playbook, bool]],
    ):
        self._name = name
        self._dependencies = dependencies
        self._components = components
        self._network = network
        self._playbooks: Dict[Type[Playbook], Tuple[Playbook, bool]] = dict(playbooks)

    def playbook(self, klass: Type[Playbook]) -> Playbook:
        if klass not in self._playbooks:
            raise KeyError(
                f"no playbook of type {klass.__name__} is registered on match "
                f"{self._name!r}"
            )
        return self._playbooks[klass][0]

    def _registration_for(
        self, klass: Type[Playbook]
    ) -> Optional[Tuple[Playbook, bool]]:
        return self._playbooks.get(klass)

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
        serialized_playbooks: List[Dict[str, Any]] = []
        for pb, exec_on_start in self._playbooks.values():
            if not hasattr(pb, "_for_ffi"):
                continue
            cfg = dict(pb._for_ffi())
            cfg["exec_on_dependency_start"] = bool(exec_on_start)
            serialized_playbooks.append(cfg)
        if serialized_playbooks:
            out["playbooks"] = serialized_playbooks
        return out


class MatchBuilder:
    def __init__(self, name: str):
        self._name = name
        self._network: Optional[str] = None
        self._dependencies: List[Any] = []
        self._components: List[Any] = []
        self._playbooks: Dict[Type[Playbook], Tuple[Playbook, bool]] = {}

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
        playbook: Playbook,
        exec_on_dependency_start: bool = False,
    ) -> "MatchBuilder":
        if not isinstance(playbook, Playbook):
            raise TypeError(
                "register_playbook requires a Playbook instance "
                f"(got {type(playbook).__name__})"
            )
        if not hasattr(playbook, "_for_ffi"):
            raise TypeError(
                "register_playbook only accepts playbooks that serialize their "
                "manifest (ManagedHttpPlaybook, ManagedMssqlPlaybook, "
                "ManagedLocalstackPlaybook, or subclasses); "
                f"{type(playbook).__name__} does not"
            )
        key = type(playbook)
        if key in self._playbooks:
            raise ValueError(
                f"a playbook of type {key.__name__} is already registered on this match"
            )
        self._playbooks[key] = (playbook, bool(exec_on_dependency_start))
        return self

    def build(self) -> Match:
        return Match(
            self._name,
            list(self._dependencies),
            list(self._components),
            self._network,
            self._playbooks,
        )
