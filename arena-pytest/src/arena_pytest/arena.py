from __future__ import annotations

import asyncio
import logging
import signal
from typing import Any, List, Optional, Type

import pytest
import pytest_asyncio

from arena_pytest.ffi._ffi import (
    ArenaBindingError,
    ArenaNativeLib,
    arena_state_document,
    close_arena,
    hard_reset as ffi_hard_reset,
    load_ffi,
    register_lifecycle_observer,
    soft_reset as ffi_soft_reset,
    unregister_lifecycle_observer,
)
from arena_pytest.lifecycle import (
    ARENA_ROOT_LOGGER_NAME,
    ArenaState,
    as_lifecycle_error,
    log_closing_summary_document,
    log_transition_document,
)
from arena_pytest.playbook import (
    PLAYBOOK_MARKER,
    ActivePlaybook,
    Playbook,
    _activate_classes,
    _class_marker_classes,
    _drop_actives,
    _finish_playbook_scope,
    _matches_from_closed_arena,
    _module_marker_classes,
    _own_marker_classes,
    _partition_classes,
    _run_managed_classes,
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

    def ffi(self) -> ArenaNativeLib:
        return self._ffi

    async def close(self) -> None:
        if not self._handle:
            return
        handle = self._handle
        token = self._dispatcher_logging_target_token
        self._handle = 0
        self._dispatcher_logging_target_token = 0
        try:
            await asyncio.to_thread(
                close_arena,
                self._ffi,
                handle,
                dispatcher_logging_target_token=token,
                on_state_document=log_closing_summary_document,
            )
        except ArenaBindingError as e:
            raise as_lifecycle_error(e) from None

    async def state(self) -> ArenaState:
        document = await asyncio.to_thread(
            arena_state_document, self._ffi, self._handle
        )
        return ArenaState.parse_json(document)

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
    __tracebackhide__ = True
    if closed_arena is None:
        pytest.skip("closed_arena fixture not overridden (no arena to open)")
    open_arena_obj = await closed_arena.open()
    yield open_arena_obj
    await open_arena_obj.close()


_previous_sigterm_handler: Any = None


def _exit_session_on_sigterm(signum: int, _frame: Any) -> None:
    logging.getLogger(ARENA_ROOT_LOGGER_NAME).info(
        "terminated by signal %d; closing arenas via session teardown", signum
    )
    pytest.exit(f"arena: terminated by signal {signum}", returncode=128 + signum)


def _install_sigterm_teardown() -> None:
    global _previous_sigterm_handler
    if _sigterm_teardown_installed():
        return
    try:
        current = signal.getsignal(signal.SIGTERM)
    except (AttributeError, ValueError):
        return
    if current not in (signal.SIG_DFL, None):
        return
    try:
        _previous_sigterm_handler = signal.signal(
            signal.SIGTERM, _exit_session_on_sigterm
        )
    except ValueError:
        _previous_sigterm_handler = None


def _sigterm_teardown_installed() -> bool:
    try:
        return signal.getsignal(signal.SIGTERM) is _exit_session_on_sigterm
    except (AttributeError, ValueError):
        return False


def _restore_sigterm_handler() -> None:
    global _previous_sigterm_handler
    if not _sigterm_teardown_installed():
        _previous_sigterm_handler = None
        return
    if _previous_sigterm_handler is None:
        _previous_sigterm_handler = signal.SIG_DFL
    try:
        signal.signal(signal.SIGTERM, _previous_sigterm_handler)
    except ValueError:
        pass
    _previous_sigterm_handler = None


_lifecycle_observer_token: int = 0
_lifecycle_observer_sessions: int = 0


def _register_transition_logging() -> None:
    global _lifecycle_observer_token, _lifecycle_observer_sessions
    _lifecycle_observer_sessions += 1
    if _lifecycle_observer_token:
        return
    ffi = load_ffi()
    if ffi is None:
        return
    _lifecycle_observer_token = register_lifecycle_observer(
        ffi, log_transition_document
    )


def _unregister_transition_logging() -> None:
    global _lifecycle_observer_token, _lifecycle_observer_sessions
    _lifecycle_observer_sessions = max(0, _lifecycle_observer_sessions - 1)
    if _lifecycle_observer_sessions or not _lifecycle_observer_token:
        return
    ffi = load_ffi()
    if ffi is not None:
        unregister_lifecycle_observer(ffi, _lifecycle_observer_token)
    _lifecycle_observer_token = 0


def pytest_unconfigure(config: pytest.Config) -> None:
    _unregister_transition_logging()
    _restore_sigterm_handler()


def pytest_configure(config: pytest.Config) -> None:
    _install_sigterm_teardown()
    _register_transition_logging()
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
_FUNCTION_MANAGED_ATTR = "_arena_pytest_function_managed_classes"


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
    unmanaged_classes, managed_classes = _partition_classes(classes)
    setattr(item, _FUNCTION_MANAGED_ATTR, managed_classes)
    if not unmanaged_classes:
        return
    request = _item_request(item)
    arena_obj = request.getfixturevalue("arena")
    matches = _matches_from_closed_arena(request.getfixturevalue("closed_arena"))
    actives = _activate_classes(arena_obj, matches, unmanaged_classes)
    setattr(item, _FUNCTION_ACTIVES_ATTR, actives)


@pytest.hookimpl(tryfirst=True)
def pytest_runtest_teardown(item: pytest.Item, nextitem: Optional[pytest.Item]) -> None:
    actives: Optional[List[ActivePlaybook]] = getattr(item, _FUNCTION_ACTIVES_ATTR, None)
    if actives:
        setattr(item, _FUNCTION_ACTIVES_ATTR, None)
        _drop_actives(actives)

    managed_classes: Optional[List[Type[Playbook]]] = getattr(
        item, _FUNCTION_MANAGED_ATTR, None
    )
    if managed_classes:
        setattr(item, _FUNCTION_MANAGED_ATTR, None)
        request = _item_request(item)
        arena_obj = request.getfixturevalue("arena")
        matches = _matches_from_closed_arena(request.getfixturevalue("closed_arena"))
        _run_managed_classes(arena_obj, matches, managed_classes)


@pytest.fixture(scope="class", autouse=True)
def _playbook_class_scope(request: pytest.FixtureRequest):
    cls = getattr(request, "cls", None)
    classes = _class_marker_classes(cls)
    if not classes:
        yield
        return
    unmanaged_classes, managed_classes = _partition_classes(classes)
    arena_obj = request.getfixturevalue("arena")
    closed = request.getfixturevalue("closed_arena")
    matches = _matches_from_closed_arena(closed)
    actives = _activate_classes(arena_obj, matches, unmanaged_classes)
    try:
        yield
    finally:
        _finish_playbook_scope(arena_obj, matches, actives, managed_classes)


@pytest.fixture(scope="module", autouse=True)
def _playbook_module_scope(request: pytest.FixtureRequest):
    module = getattr(request, "module", None)
    classes = _module_marker_classes(module)
    if not classes:
        yield
        return
    unmanaged_classes, managed_classes = _partition_classes(classes)
    arena_obj = request.getfixturevalue("arena")
    closed = request.getfixturevalue("closed_arena")
    matches = _matches_from_closed_arena(closed)
    actives = _activate_classes(arena_obj, matches, unmanaged_classes)
    try:
        yield
    finally:
        _finish_playbook_scope(arena_obj, matches, actives, managed_classes)
