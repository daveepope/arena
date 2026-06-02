from __future__ import annotations

import abc
import json
from typing import Any, Dict, List, Optional, Type, TYPE_CHECKING

import pytest

from arena_pytest.ffi._ffi import (
    ArenaBindingError,
    ArenaNativeLib,
    active_playbook_drop,
    http_playbook_verify as _ffi_http_playbook_verify,
    match_playbook_run,
    mssql_playbook_verify as _ffi_mssql_playbook_verify,
)

if TYPE_CHECKING:
    from arena_pytest.arena import OpenArena
    from arena_pytest.closed_arena import ClosedArena
    from arena_pytest.match.matches import Match


PLAYBOOK_MARKER = "playbook"


def playbook(klass: type) -> "pytest.MarkDecorator":
    return getattr(pytest.mark, PLAYBOOK_MARKER).with_args(klass)


class ActivePlaybook:
    def __init__(self, ffi: ArenaNativeLib, handle: int):
        self._ffi = ffi
        self._handle = handle
        self._body_failed = False

    def handle(self) -> int:
        return self._handle

    def _note_body_failure(self) -> None:
        self._body_failed = True

    def __enter__(self) -> "ActivePlaybook":
        return self

    def __exit__(self, exc_type, exc, tb) -> None:
        h = self._handle
        self._handle = 0
        if not h:
            return
        try:
            active_playbook_drop(self._ffi, h)
        except ArenaBindingError as e:
            if exc_type is not None or self._body_failed:
                return
            raise AssertionError(str(e)) from None


class ActiveHttpPlaybook(ActivePlaybook):
    def __init__(
        self,
        ffi: ArenaNativeLib,
        handle: int,
        dependency_identifier: str,
    ):
        super().__init__(ffi, handle)
        self._dependency_identifier = dependency_identifier

    def verify(self, method: str, url_path: str, expected_count: int) -> None:
        if not self._handle:
            raise RuntimeError(
                "ActiveHttpPlaybook.verify called after the playbook was dropped"
            )
        spec = json.dumps({
            "dependency_identifier": self._dependency_identifier,
            "method": method.upper(),
            "url_path": url_path,
            "expected_count": int(expected_count),
        })
        try:
            _ffi_http_playbook_verify(self._ffi, self._handle, spec)
        except ArenaBindingError:
            self._note_body_failure()
            raise

    def verify_at_least(self, method: str, url_path: str, minimum_count: int) -> None:
        if not self._handle:
            raise RuntimeError(
                "ActiveHttpPlaybook.verify_at_least called after the playbook was dropped"
            )
        spec = json.dumps({
            "dependency_identifier": self._dependency_identifier,
            "method": method.upper(),
            "url_path": url_path,
            "minimum_count": int(minimum_count),
        })
        try:
            _ffi_http_playbook_verify(self._ffi, self._handle, spec)
        except ArenaBindingError:
            self._note_body_failure()
            raise


class ActiveMssqlPlaybook(ActivePlaybook):
    def __init__(
        self,
        ffi: ArenaNativeLib,
        handle: int,
        dependency_identifier: str,
    ):
        super().__init__(ffi, handle)
        self._dependency_identifier = dependency_identifier

    def verify(self, query: str, expected_value: int) -> None:
        if not self._handle:
            raise RuntimeError(
                "ActiveMssqlPlaybook.verify called after the playbook was dropped"
            )
        spec = json.dumps({
            "dependency_identifier": self._dependency_identifier,
            "query": query,
            "expected_value": int(expected_value),
        })
        try:
            _ffi_mssql_playbook_verify(self._ffi, self._handle, spec)
        except ArenaBindingError:
            self._note_body_failure()
            raise


class ActiveLocalstackPlaybook(ActivePlaybook):
    pass


class Playbook(abc.ABC):
    @abc.abstractmethod
    def identifier(self) -> str:
        ...

    @abc.abstractmethod
    def run(self, arena: "OpenArena") -> ActivePlaybook:
        ...


def _resolve_playbook_classes_from_marker(mk: pytest.Mark) -> List[Type[Playbook]]:
    if len(mk.args) != 1:
        raise pytest.UsageError(
            "@pytest.mark.playbook accepts exactly one Playbook class per decorator; "
            "stack multiple @playbook(...) lines instead"
        )
    arg = mk.args[0]
    if not (isinstance(arg, type) and issubclass(arg, Playbook)):
        raise pytest.UsageError(
            f"@pytest.mark.playbook expects a Playbook subclass reference; got {arg!r}"
        )
    return [arg]


def _normalize_mark(mk):
    return getattr(mk, "mark", mk)


def _collect_marker_classes_from_iter(markers) -> List[Type[Playbook]]:
    out: List[Type[Playbook]] = []
    for mk in markers:
        out.extend(_resolve_playbook_classes_from_marker(_normalize_mark(mk)))
    return out


def _own_marker_classes(item: pytest.Item) -> List[Type[Playbook]]:
    own = getattr(item, "own_markers", None) or []
    return _collect_marker_classes_from_iter(
        mk for mk in own if _normalize_mark(mk).name == PLAYBOOK_MARKER
    )


def _class_marker_classes(cls: Optional[type]) -> List[Type[Playbook]]:
    if cls is None:
        return []
    marks = getattr(cls, "pytestmark", None) or []
    if not isinstance(marks, (list, tuple)):
        marks = [marks]
    return _collect_marker_classes_from_iter(
        mk for mk in marks if _normalize_mark(mk).name == PLAYBOOK_MARKER
    )


def _module_marker_classes(module: Optional[Any]) -> List[Type[Playbook]]:
    if module is None:
        return []
    marks = getattr(module, "pytestmark", None) or []
    if not isinstance(marks, (list, tuple)):
        marks = [marks]
    return _collect_marker_classes_from_iter(
        mk for mk in marks if _normalize_mark(mk).name == PLAYBOOK_MARKER
    )


def _resolve_playbook_for_class(
    matches: List["Match"],
    klass: Type[Playbook],
) -> tuple["Match", Playbook, bool]:
    for m in matches:
        registration = m._registration_for(klass)
        if registration is not None:
            return m, registration[0], registration[1]
    raise pytest.UsageError(
        f"@pytest.mark.playbook: no playbook of type {klass.__name__} is "
        "registered on any match (use MatchBuilder.register_playbook(...))"
    )


def _matches_from_closed_arena(closed: Optional["ClosedArena"]) -> List["Match"]:
    if closed is None:
        return []
    return list(getattr(closed, "_matches", []) or [])


def _activate_classes(
    arena: "OpenArena",
    matches: List["Match"],
    classes: List[Type[Playbook]],
) -> List[ActivePlaybook]:
    actives: List[ActivePlaybook] = []
    try:
        for klass in classes:
            _, pb, exec_on_start = _resolve_playbook_for_class(matches, klass)
            if exec_on_start:
                raise pytest.UsageError(
                    f"@pytest.mark.playbook: {klass.__name__} was registered "
                    "with exec_on_dependency_start=True and cannot be activated as a "
                    "scoped playbook"
                )
            actives.append(pb.run(arena))
    except BaseException:
        _drop_actives(actives)
        raise
    return actives


def _drop_actives(actives: List[ActivePlaybook]) -> None:
    while actives:
        a = actives.pop()
        try:
            a.__exit__(None, None, None)
        except BaseException:
            pass
