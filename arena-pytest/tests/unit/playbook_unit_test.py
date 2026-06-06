import socket
import sys
import os
from unittest.mock import MagicMock

import pytest

from ephemeral_test_runtime import RUNTIME, EphemeralTestRuntime, ephemeral_tcp_port


def _playbook_pkg():
    from arena_pytest.playbook import ActivePlaybook as _  # noqa: F401

    return sys.modules["arena_pytest.playbook"]


def test_ephemeral_tcp_port_bind_returns_nonzero_port():
    port = ephemeral_tcp_port()
    assert 1024 <= port <= 65535


def test_ephemeral_runtime_ports_are_pairwise_distinct():
    rt = EphemeralTestRuntime()
    ports = (
        rt.exec_web_app_port,
        rt.docker_web_host_port,
        rt.kafka_port,
        rt.calibration_host_port,
        rt.postgres_port,
        rt.mssql_port,
        rt.oauth_port,
        rt.localstack_host_port,
    )
    assert len(ports) == len(set(ports))


def test_ephemeral_runtime_suffixes_network_and_container_names():
    assert RUNTIME.network_name("arena-example-api-network").startswith(
        "arena-example-api-network-"
    )
    assert RUNTIME.container_name("example-api-postgres").startswith(
        "example-api-postgres-"
    )


def test_active_playbooks_for_item_without_item_attr_returns_empty_list():
    from arena_pytest.arena import active_playbooks_for_item

    item = MagicMock(spec=[])
    assert active_playbooks_for_item(item) == []


def test_active_playbooks_for_item_returns_shallow_copy_of_registered_actives():
    from arena_pytest.arena import active_playbooks_for_item
    from arena_pytest.playbook import ActivePlaybook

    active = ActivePlaybook(MagicMock(), 7)
    item = MagicMock()
    item._arena_pytest_function_actives = [active]
    first = active_playbooks_for_item(item)
    second = active_playbooks_for_item(item)
    assert first == [active]
    assert second == [active]
    assert first is not second
    first.append(MagicMock())
    assert active_playbooks_for_item(item) == [active]


def test_playbook_marker_two_args_raises_usage_error():
    from arena_pytest.playbook import _resolve_playbook_classes_from_marker

    class _TwoArgMark:
        args = ("A", "B")

    with pytest.raises(pytest.UsageError, match="exactly one Playbook class"):
        _resolve_playbook_classes_from_marker(_TwoArgMark())


def test_playbook_marker_stacked_marks_collect_both_classes():
    from arena_pytest.playbook import Playbook, _own_marker_classes, playbook

    class Alpha(Playbook):
        def identifier(self):
            return "alpha"

        def run(self, arena):
            raise NotImplementedError

    class Beta(Playbook):
        def identifier(self):
            return "beta"

        def run(self, arena):
            raise NotImplementedError

    def sample():
        pass

    sample.pytestmark = [playbook(Alpha).mark, playbook(Beta).mark]
    item = type("_Item", (), {"own_markers": list(sample.pytestmark)})()
    classes = _own_marker_classes(item)
    assert classes == [Alpha, Beta]


def test_active_playbook_drop_after_body_failure_swallows_binding_error(monkeypatch):
    from arena_pytest.ffi._ffi import ArenaBindingError
    from arena_pytest.playbook import ActivePlaybook

    monkeypatch.setattr(
        _playbook_pkg(),
        "active_playbook_drop",
        lambda _ffi, _h: (_ for _ in ()).throw(ArenaBindingError("drop expect failed")),
    )
    active = ActivePlaybook(MagicMock(), 99)
    active._note_body_failure()
    active.__exit__(None, None, None)


def test_active_playbook_drop_without_body_failure_reraises_binding_error(monkeypatch):
    from arena_pytest.ffi._ffi import ArenaBindingError
    from arena_pytest.playbook import ActivePlaybook

    monkeypatch.setattr(
        _playbook_pkg(),
        "active_playbook_drop",
        lambda _ffi, _h: (_ for _ in ()).throw(ArenaBindingError("drop expect failed")),
    )
    active = ActivePlaybook(MagicMock(), 99)
    with pytest.raises(AssertionError, match="drop expect failed"):
        active.__exit__(None, None, None)


def test_http_playbook_builder_scenario_mapping_serializes_expected_fields():
    from arena_pytest.dep.http import HttpPlaybookBuilder, ok_json

    mappings = (
        HttpPlaybookBuilder("dep-id")
        .get("/api/x")
        .in_scenario("flow")
        .will_return(ok_json({"step": 1}))
        .post("/api/y")
        .when_state_is("ready")
        .will_set_state_to("done")
        .will_return(ok_json({"ok": True}))
        .into_playbook()
        .mappings_for_ffi()
    )
    assert mappings[0]["scenario_name"] == "flow"
    assert mappings[1]["when_state_is"] == "ready"
    assert mappings[1]["will_set_state_to"] == "done"


def test_legacy_with_mapping_builds_response_spec():
    from arena_pytest.dep.http import ActiveHttpPlaybookBuilder

    with pytest.warns(DeprecationWarning):
        builder = ActiveHttpPlaybookBuilder("dep-id")
        builder.with_mapping("POST", "/api/x", 500, expect_called_at_least=1)
    mappings = builder._builder.mappings_for_ffi()
    assert mappings[0]["method"] == "POST"
    assert mappings[0]["response"]["status"] == 500
    assert mappings[0]["expect"]["kind"] == "at_least"


def test_legacy_managed_dict_mapping_serializes_flat_status():
    from arena_pytest import ManagedHttpPlaybook

    with pytest.warns(DeprecationWarning):
        playbook = ManagedHttpPlaybook(
            identifier="legacy-playbook",
            dependency_identifier="dep-id",
            mappings=[{
                "method": "POST",
                "url_path": "/api/x",
                "status": 500,
            }],
        )
    row = playbook._for_ffi()["mappings"][0]
    assert row["status"] == 500


def test_http_playbook_builder_then_return_serializes_responses_array():
    from arena_pytest.dep.http import HttpPlaybookBuilder, ok_json, server_error, status

    mappings = (
        HttpPlaybookBuilder("dep-id")
        .get("/api/x")
        .will_return(server_error())
        .then_return(status(503))
        .then_return(ok_json({"ok": True}))
        .into_playbook()
        .mappings_for_ffi()
    )
    row = mappings[0]
    assert "response" not in row
    assert row["responses"][0]["status"] == 500
    assert row["responses"][1]["status"] == 503
    assert row["responses"][2]["json_body"]["ok"] is True


def test_http_playbook_builder_will_return_in_sequence_serializes_responses_array():
    from arena_pytest.dep.http import HttpPlaybookBuilder, ok_json, server_error, status

    mappings = (
        HttpPlaybookBuilder("dep-id")
        .get("/api/x")
        .will_return_in_sequence([server_error(), status(503), ok_json({"ok": True})])
        .mappings_for_ffi()
    )
    row = mappings[0]
    assert row["responses"][0]["status"] == 500
    assert row["responses"][2]["json_body"]["ok"] is True


def test_http_playbook_builder_request_match_fields_serialize_headers_and_body_patterns():
    from arena_pytest.dep.http import HttpHeaderPattern, HttpPlaybookBuilder, ok_json

    mappings = (
        HttpPlaybookBuilder("dep-id")
        .post("/api/x")
        .with_header("Authorization", HttpHeaderPattern.matching("Bearer .+"))
        .with_request_body({"command": "ignition"})
        .with_request_body_containing("ignite")
        .with_priority(2)
        .will_return(ok_json({"accepted": True}))
        .into_playbook()
        .mappings_for_ffi()
    )
    row = mappings[0]
    assert row["priority"] == 2
    assert row["headers"]["Authorization"]["matches"] == "Bearer .+"
    assert row["body_patterns"][0]["equal_to_json"] == '{"command": "ignition"}'
    assert row["body_patterns"][1]["contains"] == "ignite"


def test_http_response_delay_and_headers_serialize_in_mapping_spec():
    from arena_pytest.dep.http import HttpPlaybookBuilder, created

    mappings = (
        HttpPlaybookBuilder("dep-id")
        .post("/api/x")
        .will_return(
            created()
            .with_header("Location", "/api/x/1")
            .with_fixed_delay_ms(30)
            .with_uniform_random_delay_ms(5, 15)
        )
        .into_playbook()
        .mappings_for_ffi()
    )
    response = mappings[0]["response"]
    assert response["status"] == 201
    assert response["headers"]["Location"] == "/api/x/1"
    assert response["fixed_delay_ms"] == 30
    assert response["delay_distribution"]["type"] == "uniform"


def test_http_playbook_builder_expect_called_serializes_expect_exactly():
    from arena_pytest.dep.http import HttpPlaybookBuilder, ok_json

    mappings = (
        HttpPlaybookBuilder("dep-id")
        .post("/api/x")
        .will_return(ok_json({"ok": True}))
        .expect_called(2)
        .into_playbook()
        .mappings_for_ffi()
    )
    assert mappings[0]["expect"] == {"kind": "exactly", "count": 2}


def test_managed_http_playbook_from_builder_preserves_fluent_mappings():
    from arena_pytest import ManagedHttpPlaybook
    from arena_pytest.dep.http import HttpPlaybookBuilder, ok_json

    playbook = ManagedHttpPlaybook.from_builder(
        "pb-from-builder",
        "dep-id",
        lambda b: b.get("/api/x").will_return(ok_json({"ok": True})),
    )
    row = playbook._for_ffi()["mappings"][0]
    assert row["method"] == "GET"
    assert row["response"]["json_body"]["ok"] is True


def test_managed_http_playbook_builder_ctor_does_not_emit_deprecation_warning():
    import warnings

    from arena_pytest import ManagedHttpPlaybook
    from arena_pytest.dep.http import HttpPlaybookBuilder, ok_json

    with warnings.catch_warnings(record=True) as recorded:
        warnings.simplefilter("always", DeprecationWarning)
        ManagedHttpPlaybook(
            identifier="pb-builder-ctor",
            dependency_identifier="dep-id",
            builder=(
                HttpPlaybookBuilder("dep-id")
                .get("/api/x")
                .will_return(ok_json({"ok": True}))
            ),
        )
    assert not any(
        w.category is DeprecationWarning for w in recorded
    )


if __name__ == "__main__":
    sys.exit(pytest.main([os.path.dirname(os.path.abspath(__file__)), "-v", "-s"]))
