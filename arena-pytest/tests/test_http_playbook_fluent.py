import json
import os
import sys

_TESTS_DIR = os.path.dirname(os.path.abspath(__file__))
if _TESTS_DIR not in sys.path:
    sys.path.insert(0, _TESTS_DIR)

import pytest
import requests

from arena_pytest.dep.http import (
    HttpDependencyBuilder,
    HttpHeaderPattern,
    HttpPlaybookBuilder,
    ok_json,
    server_error,
    status,
)
from arena_pytest.ffi._ffi import close_arena, load_ffi, open_arena
from readings_ephemeral_test_runtime import ephemeral_tcp_port


def _open_fluent_http_arena():
    ffi = load_ffi()
    ports = [ephemeral_tcp_port() for _ in range(5)]
    last_err = None
    for port in ports:
        dep = HttpDependencyBuilder("fluent-http").with_port(port).build()
        config = json.dumps(
            {
                "match_name": "fluent-http-match",
                "dependencies": [dep._for_ffi()],
            }
        )
        try:
            arena_h = open_arena(ffi, b"fluent-http-arena", config)
            return ffi, arena_h, dep, port
        except Exception as err:
            last_err = err
            msg = str(err)
            if "ports are not available" not in msg and "StartContainer" not in msg:
                raise
    raise last_err


@pytest.fixture(scope="module")
def _fluent_http_arena():
    ffi, arena_h, dep, port = _open_fluent_http_arena()
    try:
        yield ffi, arena_h, dep, port
    finally:
        close_arena(ffi, arena_h)


class _FluentArena:
    def __init__(self, ffi, handle):
        self._ffi = ffi
        self._handle = handle


def test_http_playbook_fluent_sequence_then_return_returns_statuses_in_order(
    _fluent_http_arena,
):
    ffi, arena_h, dep, port = _fluent_http_arena
    arena = _FluentArena(ffi, arena_h)
    url = f"http://127.0.0.1:{port}/api/telemetry/altitude"
    with (
        HttpPlaybookBuilder(dep.identifier)
        .get("/api/telemetry/altitude")
        .will_return(server_error())
        .then_return(status(503))
        .then_return(ok_json({"altitude_km": 185}))
        .open(arena)
    ):
        assert requests.get(url, timeout=10).status_code == 500
        assert requests.get(url, timeout=10).status_code == 503
        resp = requests.get(url, timeout=10)
        assert resp.status_code == 200
        assert resp.json()["altitude_km"] == 185


def test_http_playbook_fluent_scenario_state_transitions_return_expected_bodies(
    _fluent_http_arena,
):
    ffi, arena_h, dep, port = _fluent_http_arena
    arena = _FluentArena(ffi, arena_h)
    base = f"http://127.0.0.1:{port}"
    with (
        HttpPlaybookBuilder(dep.identifier)
        .get("/api/vehicle/telemetry")
        .in_scenario("saturn-v-launch")
        .will_return(ok_json({"stage": "terminal-count"}))
        .post("/api/vehicle/main-engine-start")
        .in_scenario("saturn-v-launch")
        .will_set_state_to("first-stage-flight")
        .will_return(ok_json({"stage": "main-engine-start"}))
        .get("/api/vehicle/telemetry")
        .in_scenario("saturn-v-launch")
        .when_state_is("first-stage-flight")
        .will_return(ok_json({"stage": "first-stage-flight"}))
        .open(arena)
    ):
        assert requests.get(f"{base}/api/vehicle/telemetry", timeout=10).json()["stage"] == "terminal-count"
        assert (
            requests.post(
                f"{base}/api/vehicle/main-engine-start",
                json={"command": "ignition"},
                timeout=10,
            ).json()["stage"]
            == "main-engine-start"
        )
        assert requests.get(f"{base}/api/vehicle/telemetry", timeout=10).json()["stage"] == "first-stage-flight"


def test_http_playbook_fluent_request_body_and_header_match_returns_stubbed_response(
    _fluent_http_arena,
):
    ffi, arena_h, dep, port = _fluent_http_arena
    arena = _FluentArena(ffi, arena_h)
    url = f"http://127.0.0.1:{port}/api/vehicle/ignite"
    with (
        HttpPlaybookBuilder(dep.identifier)
        .post("/api/vehicle/ignite")
        .with_header("Authorization", HttpHeaderPattern.equal_to("Bearer launch-token"))
        .with_request_body({"command": "ignition"})
        .will_return(ok_json({"accepted": True}))
        .open(arena)
    ):
        resp = requests.post(
            url,
            json={"command": "ignition"},
            headers={"Authorization": "Bearer launch-token"},
            timeout=10,
        )
        assert resp.status_code == 200
        assert resp.json()["accepted"] is True
