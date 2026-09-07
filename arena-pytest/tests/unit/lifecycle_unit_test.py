import contextlib
import dataclasses
import json
import logging

import pytest

from arena_pytest.ffi._ffi import ArenaBindingError
from arena_pytest.lifecycle import (
    ArenaLifecycleError,
    ArenaState,
    ComponentState,
    DependencyState,
    Fault,
    arena_logger_name,
    as_lifecycle_error,
    log_closing_summary,
    log_closing_summary_document,
    log_transition,
    log_transition_document,
)

FIXTURE_STATE_JSON = json.dumps(
    {
        "id": "orders",
        "state": "arena_faulted",
        "at": "2026-09-06T11:02:44.812Z",
        "dependencies": [
            {
                "id": "orders-postgres",
                "state": "faulted",
                "faults": [
                    {
                        "id": "orders-postgres",
                        "subject": "dependency",
                        "message": "failed to start",
                        "at": "2026-09-06T11:02:44.801Z",
                        "faults": [
                            {
                                "id": "orders-postgres",
                                "subject": "dependency",
                                "message": "connection refused on 127.0.0.1:5432",
                                "at": "2026-09-06T11:02:44.799Z",
                                "faults": [],
                            }
                        ],
                    }
                ],
                "children": [
                    {
                        "id": "orders-postgres-seed",
                        "state": "stopped",
                        "faults": [],
                        "children": [],
                    }
                ],
            }
        ],
        "components": [
            {"id": "orders-api", "state": "not_started", "faults": [], "children": []}
        ],
        "faults": [
            {
                "id": "orders-postgres",
                "subject": "dependency",
                "message": "failed to start",
                "at": "2026-09-06T11:02:44.801Z",
                "faults": [],
            }
        ],
    }
)


class _LineCapture(logging.Handler):
    def __init__(self) -> None:
        super().__init__()
        self.lines: list[tuple[int, str]] = []

    def emit(self, record: logging.LogRecord) -> None:
        self.lines.append((record.levelno, record.getMessage()))


@contextlib.contextmanager
def _capture(logger_name: str):
    lg = logging.getLogger(logger_name)
    capture = _LineCapture()
    previous_level = lg.level
    lg.addHandler(capture)
    lg.setLevel(logging.DEBUG)
    try:
        yield capture
    finally:
        lg.removeHandler(capture)
        lg.setLevel(previous_level)


def test_parse_json_fixture_document_round_trips_every_field():
    state = ArenaState.parse_json(FIXTURE_STATE_JSON)

    assert state.id == "orders"
    assert state.state == "arena_faulted"
    assert state.at == "2026-09-06T11:02:44.812Z"
    assert state.dependencies[0].id == "orders-postgres"
    assert state.dependencies[0].state == "faulted"
    assert state.dependencies[0].children[0].id == "orders-postgres-seed"
    assert state.components[0].state == "not_started"
    assert state.faults[0].subject == "dependency"
    cause = state.dependencies[0].faults[0].faults[0]
    assert cause.message == "connection refused on 127.0.0.1:5432"
    assert cause.at == "2026-09-06T11:02:44.799Z"


def test_parse_json_non_object_document_raises_value_error():
    with pytest.raises(ValueError):
        ArenaState.parse_json("[1, 2]")


def test_parse_missing_optional_collections_defaults_to_empty():
    state = ArenaState.parse({"id": "bare", "state": "arena_created", "at": "t"})

    assert state.dependencies == ()
    assert state.components == ()
    assert state.faults == ()


def test_is_faulted_faulted_token_returns_true():
    assert ArenaState.parse_json(FIXTURE_STATE_JSON).is_faulted()
    assert not ArenaState.parse(
        {"id": "x", "state": "arena_open", "at": "t"}
    ).is_faulted()


def test_dependency_nested_child_identifier_returns_that_child():
    state = ArenaState.parse_json(FIXTURE_STATE_JSON)

    child = state.dependency("orders-postgres-seed")

    assert child is not None
    assert child.state == "stopped"
    assert state.dependency("nope") is None


def test_component_top_level_identifier_returns_that_component():
    state = ArenaState.parse_json(FIXTURE_STATE_JSON)

    assert state.component("orders-api").state == "not_started"


def test_arena_state_parsed_instance_is_frozen():
    state = ArenaState.parse_json(FIXTURE_STATE_JSON)

    with pytest.raises(dataclasses.FrozenInstanceError):
        state.id = "other"


@pytest.mark.parametrize(
    "arena_id,expected",
    [
        ("orders", "arena.orders"),
        ("orders.v2", "arena.orders_v2"),
        ("  ", "arena"),
        ("", "arena"),
    ],
)
def test_arena_logger_name_identifier_matches_dispatcher_namespace(arena_id, expected):
    assert arena_logger_name(arena_id) == expected


def test_as_lifecycle_error_state_document_returns_lifecycle_error_with_state():
    error = ArenaBindingError("open failed")
    error.state_document = FIXTURE_STATE_JSON

    converted = as_lifecycle_error(error)

    assert isinstance(converted, ArenaLifecycleError)
    assert str(converted) == "open failed"
    assert converted.state.id == "orders"


def test_as_lifecycle_error_no_state_document_returns_original_error():
    error = ArenaBindingError("plain failure")

    assert as_lifecycle_error(error) is error


def test_as_lifecycle_error_unparseable_document_returns_original_error():
    error = ArenaBindingError("open failed")
    error.state_document = "{not json"

    assert as_lifecycle_error(error) is error


def test_as_lifecycle_error_lifecycle_error_returns_it_unchanged():
    error = ArenaLifecycleError("already converted")

    assert as_lifecycle_error(error) is error


def test_log_transition_clean_state_logs_info_under_arena_logger():
    state = ArenaState.parse(
        {"id": "transition-clean", "state": "dependencies_starting", "at": "t"}
    )

    with _capture("arena.transition-clean") as capture:
        log_transition(state)

    assert capture.lines == [(logging.INFO, "dependencies_starting")]


def test_log_transition_faulted_state_logs_error_with_fault_count():
    state = ArenaState.parse_json(FIXTURE_STATE_JSON)

    with _capture("arena.orders") as capture:
        log_transition(state)

    assert capture.lines == [(logging.ERROR, "arena_faulted | faults=1")]


def test_log_closing_summary_state_logs_token_and_fault_count():
    state = ArenaState.parse(
        {"id": "closing-summary", "state": "arena_closed", "at": "t"}
    )

    with _capture("arena.closing-summary") as capture:
        log_closing_summary(state)

    assert capture.lines == [
        (logging.INFO, "closing summary | state=arena_closed, faults=0")
    ]


def test_log_transition_document_valid_document_logs_the_transition():
    with _capture("arena.doc-valid") as capture:
        log_transition_document(
            json.dumps({"id": "doc-valid", "state": "arena_open", "at": "t"})
        )

    assert capture.lines == [(logging.INFO, "arena_open")]


def test_log_transition_document_unparseable_document_warns_on_root_logger():
    with _capture("arena") as capture:
        log_transition_document("{broken")

    assert capture.lines
    assert capture.lines[0][0] == logging.WARNING


def test_log_closing_summary_document_valid_document_logs_the_summary():
    with _capture("arena.close-doc-valid") as capture:
        log_closing_summary_document(
            json.dumps({"id": "close-doc-valid", "state": "arena_closed", "at": "t"})
        )

    assert capture.lines == [
        (logging.INFO, "closing summary | state=arena_closed, faults=0")
    ]


def test_log_closing_summary_document_unparseable_document_warns_on_root_logger():
    with _capture("arena") as capture:
        log_closing_summary_document("{broken")

    assert capture.lines
    assert capture.lines[0][0] == logging.WARNING
