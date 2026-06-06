from __future__ import annotations

import asyncio
from typing import Any, List, Optional

import pytest
import pytest_asyncio

from arena_pytest.ffi._ffi import (
    ArenaBindingError,
    ArenaNativeLib,
    close_arena,
    hard_reset as ffi_hard_reset,
    load_ffi,
    soft_reset as ffi_soft_reset,
)
from arena_pytest.playbook import (
    PLAYBOOK_MARKER,
    ActivePlaybook,
    _activate_classes,
    _class_marker_classes,
    _drop_actives,
    _matches_from_closed_arena,
    _module_marker_classes,
    _own_marker_classes,
)


class OpenArena:
    def __init__(
        self,
        ffi: ArenaNativeLib,
        handle: int,
        dispatcher_logging_target_token: int = 0,
    ):
        self._ffi = ffi
        self._handle = handle
        self._dispatcher_logging_target_token = dispatcher_logging_target_token

    def handle(self) -> int:
        return self._handle

    async def close(self) -> None:
        if self._handle:
            await asyncio.to_thread(
                close_arena,
                self._ffi,
                self._handle,
                dispatcher_logging_target_token=self._dispatcher_logging_target_token,
            )
            self._handle = 0
            self._dispatcher_logging_target_token = 0

    async def soft_reset(self, dependency_identifier: str) -> None:
        await asyncio.to_thread(
            ffi_soft_reset, self._ffi, self._handle, dependency_identifier
        )

    async def hard_reset(self, dependency_identifier: str) -> None:
        await asyncio.to_thread(
            ffi_hard_reset, self._ffi, self._handle, dependency_identifier
        )


@pytest.fixture(scope="session")
def arena_ffi() -> ArenaNativeLib:
    ffi = load_ffi()
    if ffi is None:
        pytest.skip(
            "arena shared library not found (set ARENA_FFI_LIB or use Bazel runfiles)"
        )
    return ffi


@pytest.fixture(scope="session")
def closed_arena() -> Optional[Any]:
    return None


@pytest_asyncio.fixture(scope="session")
async def arena(closed_arena) -> OpenArena:
    if closed_arena is None:
        pytest.skip("closed_arena fixture not overridden (no arena to open)")
    try:
        open_arena_obj = await closed_arena.open()
    except ArenaBindingError as e:
        pytest.fail(f"arena_open failed: {e}", pytrace=False)
    yield open_arena_obj
    await open_arena_obj.close()


def pytest_configure(config: pytest.Config) -> None:
    config.addinivalue_line(
        "markers",
        f"{PLAYBOOK_MARKER}(klass): open one playbook identified by its Playbook "
        "subclass for the test, class, or module scope; stack multiple "
        "@playbook(...) decorators for more than one.",
    )
    if config.getini("asyncio_mode") in (None, "strict", "STRICT"):
        config._inicache["asyncio_mode"] = "auto"
    if not config.getini("asyncio_default_fixture_loop_scope"):
        config._inicache["asyncio_default_fixture_loop_scope"] = "session"


_FUNCTION_ACTIVES_ATTR = "_arena_pytest_function_actives"


def active_playbooks_for_item(item: pytest.Item) -> List[ActivePlaybook]:
    actives: Optional[List[ActivePlaybook]] = getattr(item, _FUNCTION_ACTIVES_ATTR, None)
    return list(actives) if actives else []


def _item_request(item: pytest.Item) -> pytest.FixtureRequest:
    request = getattr(item, "_request", None)
    if request is None:
        raise pytest.UsageError(
            "@pytest.mark.playbook requires a pytest test function with the "
            "'arena' fixture available"
        )
    return request


@pytest.hookimpl(trylast=True)
def pytest_runtest_setup(item: pytest.Item) -> None:
    classes = _own_marker_classes(item)
    if not classes:
        return
    request = _item_request(item)
    arena_obj = request.getfixturevalue("arena")
    matches = _matches_from_closed_arena(request.getfixturevalue("closed_arena"))
    actives = _activate_classes(arena_obj, matches, classes)
    setattr(item, _FUNCTION_ACTIVES_ATTR, actives)


@pytest.hookimpl(tryfirst=True)
def pytest_runtest_teardown(item: pytest.Item, nextitem: Optional[pytest.Item]) -> None:
    actives: Optional[List[ActivePlaybook]] = getattr(item, _FUNCTION_ACTIVES_ATTR, None)
    if not actives:
        return
    setattr(item, _FUNCTION_ACTIVES_ATTR, None)
    _drop_actives(actives)


@pytest.fixture(scope="class", autouse=True)
def _playbook_class_scope(request: pytest.FixtureRequest):
    cls = getattr(request, "cls", None)
    classes = _class_marker_classes(cls)
    if not classes:
        yield
        return
    arena_obj = request.getfixturevalue("arena")
    closed = request.getfixturevalue("closed_arena")
    matches = _matches_from_closed_arena(closed)
    actives = _activate_classes(arena_obj, matches, classes)
    try:
        yield
    finally:
        _drop_actives(actives)


@pytest.fixture(scope="module", autouse=True)
def _playbook_module_scope(request: pytest.FixtureRequest):
    module = getattr(request, "module", None)
    classes = _module_marker_classes(module)
    if not classes:
        yield
        return
    arena_obj = request.getfixturevalue("arena")
    closed = request.getfixturevalue("closed_arena")
    matches = _matches_from_closed_arena(closed)
    actives = _activate_classes(arena_obj, matches, classes)
    try:
        yield
    finally:
        _drop_actives(actives)
