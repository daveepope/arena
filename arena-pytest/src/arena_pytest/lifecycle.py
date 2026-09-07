from __future__ import annotations

import json
import logging
from dataclasses import dataclass
from typing import Any, Mapping, Optional, Tuple

from arena_pytest.ffi._ffi import ArenaBindingError

ARENA_ROOT_LOGGER_NAME = "arena"

ARENA_FAULTED = "arena_faulted"
ARENA_CLOSED = "arena_closed"


@dataclass(frozen=True)
class Fault:
    id: str
    subject: str
    message: str
    at: str
    faults: Tuple["Fault", ...] = ()

    @classmethod
    def parse(cls, data: Mapping[str, Any]) -> "Fault":
        return cls(
            id=str(data.get("id", "")),
            subject=str(data.get("subject", "")),
            message=str(data.get("message", "")),
            at=str(data.get("at", "")),
            faults=_parse_faults(data.get("faults")),
        )


@dataclass(frozen=True)
class DependencyState:
    id: str
    state: str
    faults: Tuple[Fault, ...] = ()
    children: Tuple["DependencyState", ...] = ()

    @classmethod
    def parse(cls, data: Mapping[str, Any]) -> "DependencyState":
        return cls(
            id=str(data.get("id", "")),
            state=str(data.get("state", "")),
            faults=_parse_faults(data.get("faults")),
            children=tuple(
                cls.parse(child) for child in data.get("children") or () if isinstance(child, Mapping)
            ),
        )


@dataclass(frozen=True)
class ComponentState:
    id: str
    state: str
    faults: Tuple[Fault, ...] = ()
    children: Tuple["ComponentState", ...] = ()

    @classmethod
    def parse(cls, data: Mapping[str, Any]) -> "ComponentState":
        return cls(
            id=str(data.get("id", "")),
            state=str(data.get("state", "")),
            faults=_parse_faults(data.get("faults")),
            children=tuple(
                cls.parse(child) for child in data.get("children") or () if isinstance(child, Mapping)
            ),
        )


@dataclass(frozen=True)
class ArenaState:
    id: str
    state: str
    at: str
    dependencies: Tuple[DependencyState, ...] = ()
    components: Tuple[ComponentState, ...] = ()
    faults: Tuple[Fault, ...] = ()

    @classmethod
    def parse(cls, data: Mapping[str, Any]) -> "ArenaState":
        return cls(
            id=str(data.get("id", "")),
            state=str(data.get("state", "")),
            at=str(data.get("at", "")),
            dependencies=tuple(
                DependencyState.parse(dep)
                for dep in data.get("dependencies") or ()
                if isinstance(dep, Mapping)
            ),
            components=tuple(
                ComponentState.parse(comp)
                for comp in data.get("components") or ()
                if isinstance(comp, Mapping)
            ),
            faults=_parse_faults(data.get("faults")),
        )

    @classmethod
    def parse_json(cls, document: str) -> "ArenaState":
        data = json.loads(document)
        if not isinstance(data, Mapping):
            raise ValueError("arena state document must be a json object")
        return cls.parse(data)

    def is_faulted(self) -> bool:
        return self.state == ARENA_FAULTED

    def dependency(self, identifier: str) -> Optional[DependencyState]:
        return _find_subject(self.dependencies, identifier)

    def component(self, identifier: str) -> Optional[ComponentState]:
        return _find_subject(self.components, identifier)


def _parse_faults(raw: Any) -> Tuple[Fault, ...]:
    if not raw:
        return ()
    return tuple(Fault.parse(item) for item in raw if isinstance(item, Mapping))


def _find_subject(subjects, identifier: str):
    for subject in subjects:
        if subject.id == identifier:
            return subject
        found = _find_subject(subject.children, identifier)
        if found is not None:
            return found
    return None


class ArenaLifecycleError(ArenaBindingError):
    def __init__(self, message: str, state: Optional[ArenaState] = None):
        super().__init__(message)
        self.state = state


def as_lifecycle_error(error: ArenaBindingError) -> ArenaBindingError:
    if isinstance(error, ArenaLifecycleError):
        return error
    document = getattr(error, "state_document", None)
    if not document:
        return error
    try:
        state = ArenaState.parse_json(document)
    except (ValueError, TypeError):
        return error
    return ArenaLifecycleError(str(error), state)


def arena_logger_name(arena_id: str) -> str:
    segment = arena_id.strip().replace(".", "_")
    if not segment:
        return ARENA_ROOT_LOGGER_NAME
    return f"{ARENA_ROOT_LOGGER_NAME}.{segment}"


def log_transition(state: ArenaState) -> None:
    lg = logging.getLogger(arena_logger_name(state.id))
    level = logging.ERROR if state.is_faulted() else logging.INFO
    fault_count = len(state.faults)
    if fault_count:
        lg.log(level, "%s | faults=%d", state.state, fault_count)
    else:
        lg.log(level, "%s", state.state)


def log_closing_summary(state: ArenaState) -> None:
    lg = logging.getLogger(arena_logger_name(state.id))
    lg.info("closing summary | state=%s, faults=%d", state.state, len(state.faults))


def log_closing_summary_document(document: str) -> None:
    try:
        state = ArenaState.parse_json(document)
    except (ValueError, TypeError):
        logging.getLogger(ARENA_ROOT_LOGGER_NAME).warning(
            "unparseable arena closing state: %s", document
        )
        return
    log_closing_summary(state)


def log_transition_document(document: str) -> None:
    try:
        state = ArenaState.parse_json(document)
    except (ValueError, TypeError):
        logging.getLogger(ARENA_ROOT_LOGGER_NAME).warning(
            "unparseable arena state transition: %s", document
        )
        return
    log_transition(state)
