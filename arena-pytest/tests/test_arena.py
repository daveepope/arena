import json
import os
import queue
import sys

_TESTS_DIR = os.path.dirname(os.path.abspath(__file__))
if _TESTS_DIR not in sys.path:
    sys.path.insert(0, _TESTS_DIR)

import threading
import time
from typing import Any

import pytest
import requests

from arena_pytest import ArenaBindingError, playbook
from arena_pytest.dep.http import HttpDependencyBuilder
from arena_pytest.ffi._ffi import (
    active_playbook_drop,
    close_arena,
    http_playbook_open,
    http_playbook_verify,
    load_ffi,
    open_arena,
)

from playbooks import (
    CalibrationOutagePlaybook,
    ResetValidationDbPlaybook,
)
from readings_arena_config import (
    CALIBRATION_VALIDATE_PATH,
    DOCKER_WEB_HOST_PORT,
    EXEC_WEB_APP_PORT,
    KAFKA_CONSUMER_GROUP_LABEL,
    KAFKA_PORT,
    KAFKA_TOPIC,
)
from readings_ephemeral_test_runtime import ephemeral_tcp_port

BASE_URL_EXEC = f"http://127.0.0.1:{EXEC_WEB_APP_PORT}"
BASE_URL_DOCKER = f"http://127.0.0.1:{DOCKER_WEB_HOST_PORT}"


def _auth_headers(access_token: str) -> dict[str, str]:
    return {"Authorization": f"Bearer {access_token}"}


@playbook(ResetValidationDbPlaybook)
def test_create_reading_publishes_kafka_event_and_lists_via_http(
    arena, oauth_access_token
):
    bootstrap = f"localhost:{KAFKA_PORT}"
    id_queue: queue.Queue[int] = queue.Queue()
    result_holder: list[Any] = []

    consumer_thread = threading.Thread(
        target=_run_reading_created_consumer,
        args=(bootstrap, KAFKA_TOPIC, id_queue, result_holder, "exec"),
    )
    consumer_thread.start()

    created_id = _create_reading(
        BASE_URL_EXEC,
        "Readings API User",
        77,
        "sqs happy path",
        oauth_access_token,
    )
    id_queue.put(created_id)

    consumer_thread.join(timeout=10)
    assert len(result_holder) == 1, "consumer should have completed"
    consumed = result_holder[0]
    if isinstance(consumed, Exception):
        raise consumed
    assert consumed["id"] == created_id
    assert consumed["user_name"] == "Readings API User"
    assert consumed["value"] == 77
    assert consumed.get("comment") == "sqs happy path"

    readings = _get_readings(BASE_URL_EXEC, oauth_access_token)
    found = next((r for r in readings if r["id"] == created_id), None)
    assert found is not None, "should find newly created reading"
    assert found["id"] == created_id
    assert found["user_name"] == "Readings API User"
    assert found["value"] == 77
    assert found.get("comment") == "sqs happy path"


@playbook(ResetValidationDbPlaybook)
def test_create_multiple_readings_are_listed(arena, oauth_access_token):
    id1 = _create_reading(BASE_URL_EXEC, "Bending", 1, "", oauth_access_token)
    id2 = _create_reading(
        BASE_URL_EXEC,
        "joe",
        2,
        "We're going to need a bigger ship",
        oauth_access_token,
    )
    readings = _get_readings(BASE_URL_EXEC, oauth_access_token)
    ids = {r["id"] for r in readings}
    assert id1 in ids
    assert id2 in ids


def test_post_reading_returns_500_when_calibration_api_returns_500(
    arena, calibration_outage_playbook, oauth_access_token
):
    with calibration_outage_playbook.run(arena):
        r = requests.post(
            f"{BASE_URL_EXEC}/readings",
            json={"user_name": "Outage Test User", "value": 99, "comment": None},
            headers=_auth_headers(oauth_access_token),
            timeout=10,
        )
        assert r.status_code == 500, (
            f"expected 500 while calibration is in outage playbook, got {r.status_code}: {r.text}"
        )

    recovered_id = _create_reading(
        BASE_URL_EXEC, "Recovery Test User", 17, "post-outage", oauth_access_token
    )
    readings = _get_readings(BASE_URL_EXEC, oauth_access_token)
    found = next((r for r in readings if r["id"] == recovered_id), None)
    assert found is not None, "recovered reading should be present"
    assert found["user_name"] == "Recovery Test User"
    assert found["value"] == 17


@playbook(CalibrationOutagePlaybook)
@playbook(ResetValidationDbPlaybook)
def test_post_reading_returns_500_when_calibration_api_overridden_by_playbook(
    arena, oauth_access_token
):
    r = requests.post(
        f"{BASE_URL_EXEC}/readings",
        json={"user_name": "Marker Outage", "value": 1, "comment": None},
        headers=_auth_headers(oauth_access_token),
        timeout=10,
    )
    assert r.status_code == 500, (
        f"expected 500 while marker-scoped calibration outage is active, "
        f"got {r.status_code}: {r.text}"
    )


def test_post_reading_returns_200_after_marker_scope_exits(arena, oauth_access_token):
    recovered_id = _create_reading(
        BASE_URL_EXEC, "Marker Recovery", 2, None, oauth_access_token
    )
    readings = _get_readings(BASE_URL_EXEC, oauth_access_token)
    assert any(r["id"] == recovered_id for r in readings)


@playbook(ResetValidationDbPlaybook)
def test_create_reading_with_validation_db_scoped_playbook(
    arena, oauth_access_token
):
    created_id = _create_reading(
        BASE_URL_EXEC,
        "Validation DB Scoped",
        7,
        "mssql scope",
        oauth_access_token,
    )
    readings = _get_readings(BASE_URL_EXEC, oauth_access_token)
    assert any(r["id"] == created_id for r in readings)


@playbook(CalibrationOutagePlaybook)
@playbook(ResetValidationDbPlaybook)
def test_post_reading_returns_500_under_scoped_playbook_stack(arena, oauth_access_token):
    r = requests.post(
        f"{BASE_URL_EXEC}/readings",
        json={"user_name": "Marker Stack Outage", "value": 1, "comment": None},
        headers=_auth_headers(oauth_access_token),
        timeout=10,
    )
    assert r.status_code == 500, (
        f"expected 500 while marker-stack outage is active, "
        f"got {r.status_code}: {r.text}"
    )


def test_active_http_playbook_verify_count_mismatch_raises(
    arena, calibration_outage_verify_probe_playbook
):
    with calibration_outage_verify_probe_playbook.run(arena) as active:
        with pytest.raises(ArenaBindingError):
            active.verify("POST", CALIBRATION_VALIDATE_PATH, 1)


def test_active_http_playbook_verify_at_least_succeeds_with_traffic(
    arena, calibration_outage_verify_probe_playbook, oauth_access_token
):
    with calibration_outage_verify_probe_playbook.run(arena) as active:
        requests.post(
            f"{BASE_URL_EXEC}/readings",
            json={"user_name": "Verify At Least", "value": 3, "comment": None},
            headers=_auth_headers(oauth_access_token),
            timeout=10,
        )
        active.verify_at_least("POST", CALIBRATION_VALIDATE_PATH, 1)


def test_active_http_playbook_failed_verify_drop_does_not_raise(
    arena, calibration_outage_verify_probe_playbook
):
    with calibration_outage_verify_probe_playbook.run(arena) as active:
        with pytest.raises(ArenaBindingError):
            active.verify("POST", CALIBRATION_VALIDATE_PATH, 1)


def test_active_http_playbook_verify_at_least_without_traffic_raises(
    arena, calibration_outage_verify_probe_playbook
):
    with calibration_outage_verify_probe_playbook.run(arena) as active:
        with pytest.raises(ArenaBindingError):
            active.verify_at_least("POST", CALIBRATION_VALIDATE_PATH, 1)


def _open_http_playbook_ffi_arena():
    port = ephemeral_tcp_port()
    dep = HttpDependencyBuilder("ffi-open-verify").with_port(port).build()
    config = json.dumps(
        {
            "match_name": "ffi-http-match",
            "dependencies": [dep._for_ffi()],
        }
    )
    ffi = load_ffi()
    arena_h = open_arena(ffi, b"ffi-http-arena", config)
    return ffi, arena_h, dep, port


def test_http_playbook_ffi_open_verify_at_least_with_traffic():
    ffi, arena_h, dep, port = _open_http_playbook_ffi_arena()
    dep_id = dep.identifier
    try:
        open_spec = json.dumps(
            {
                "dependency_identifier": dep_id,
                "mappings": [
                    {
                        "method": "GET",
                        "url_path": "/api/ffi/playbook",
                        "response": {"status": 200, "json_body": {"ok": True}},
                    }
                ],
            }
        )
        pb_h = http_playbook_open(ffi, arena_h, open_spec)
        url = f"http://127.0.0.1:{port}/api/ffi/playbook"
        assert requests.get(url, timeout=10).status_code == 200
        verify_spec = json.dumps(
            {
                "method": "GET",
                "url_path": "/api/ffi/playbook",
                "minimum_count": 1,
            }
        )
        http_playbook_verify(ffi, pb_h, verify_spec)
        active_playbook_drop(ffi, pb_h)
    finally:
        close_arena(ffi, arena_h)


def test_http_playbook_ffi_verify_expected_count_without_traffic_raises():
    ffi, arena_h, dep, _port = _open_http_playbook_ffi_arena()
    dep_id = dep.identifier
    try:
        open_spec = json.dumps(
            {
                "dependency_identifier": dep_id,
                "mappings": [
                    {
                        "method": "GET",
                        "url_path": "/api/ffi/playbook",
                        "response": {"status": 200, "json_body": {"ok": True}},
                    }
                ],
            }
        )
        pb_h = http_playbook_open(ffi, arena_h, open_spec)
        verify_spec = json.dumps(
            {
                "method": "GET",
                "url_path": "/api/ffi/playbook",
                "expected_count": 1,
            }
        )
        with pytest.raises(ArenaBindingError):
            http_playbook_verify(ffi, pb_h, verify_spec)
        active_playbook_drop(ffi, pb_h)
    finally:
        close_arena(ffi, arena_h)


def test_http_playbook_ffi_verify_both_count_fields_raises():
    ffi, arena_h, dep, _port = _open_http_playbook_ffi_arena()
    dep_id = dep.identifier
    try:
        open_spec = json.dumps(
            {
                "dependency_identifier": dep_id,
                "mappings": [
                    {
                        "method": "GET",
                        "url_path": "/api/ffi/playbook",
                        "response": {"status": 200, "json_body": {"ok": True}},
                    }
                ],
            }
        )
        pb_h = http_playbook_open(ffi, arena_h, open_spec)
        verify_spec = json.dumps(
            {
                "method": "GET",
                "url_path": "/api/ffi/playbook",
                "expected_count": 1,
                "minimum_count": 1,
            }
        )
        with pytest.raises(ArenaBindingError):
            http_playbook_verify(ffi, pb_h, verify_spec)
        active_playbook_drop(ffi, pb_h)
    finally:
        close_arena(ffi, arena_h)


@playbook(ResetValidationDbPlaybook)
def test_containerized_app_create_reading_publishes_kafka_event(
    docker_web_enabled,
    arena_docker,
    oauth_access_token,
):
    if not docker_web_enabled:
        pytest.skip(
            "No containerized web app: OAuth issuer is loopback-only "
            "(JWT issuer cannot be reached from a bridge-network container)."
        )

    bootstrap = f"localhost:{KAFKA_PORT}"
    id_queue: queue.Queue[int] = queue.Queue()
    result_holder: list[Any] = []

    consumer_thread = threading.Thread(
        target=_run_reading_created_consumer,
        args=(bootstrap, KAFKA_TOPIC, id_queue, result_holder, "docker"),
    )
    consumer_thread.start()

    created_id = _create_reading(
        BASE_URL_DOCKER,
        "Docker Test User",
        42,
        "test comment",
        oauth_access_token,
    )
    id_queue.put(created_id)

    consumer_thread.join(timeout=10)
    assert len(result_holder) == 1, "consumer should have completed"
    consumed = result_holder[0]
    if isinstance(consumed, Exception):
        raise consumed
    assert consumed["id"] == created_id
    assert consumed["user_name"] == "Docker Test User"
    assert consumed["value"] == 42
    assert consumed.get("comment") == "test comment"

    readings = _get_readings(BASE_URL_DOCKER, oauth_access_token)
    found = next((r for r in readings if r["id"] == created_id), None)
    assert found is not None, "should find newly created reading"
    assert found["id"] == created_id
    assert found["user_name"] == "Docker Test User"
    assert found["value"] == 42
    assert found.get("comment") == "test comment"


def _get_readings(base_url: str, access_token: str) -> list:
    r = requests.get(
        f"{base_url}/readings",
        headers=_auth_headers(access_token),
        timeout=10,
    )
    r.raise_for_status()
    return r.json()


def _create_reading(
    base_url: str,
    user_name: str,
    value: int,
    comment: str | None,
    access_token: str,
) -> int:
    r = requests.post(
        f"{base_url}/readings",
        json={"user_name": user_name, "value": value, "comment": comment},
        headers=_auth_headers(access_token),
        timeout=10,
    )
    r.raise_for_status()
    return r.json()["id"]


def _new_readings_kafka_consumer(bootstrap: str, topic: str, group_prefix: str) -> Any:
    from kafka import KafkaConsumer

    return KafkaConsumer(
        topic,
        bootstrap_servers=bootstrap,
        group_id=f"{KAFKA_CONSUMER_GROUP_LABEL}-{group_prefix}-{os.getpid()}",
        auto_offset_reset="earliest",
    )


def _run_reading_created_consumer(
    bootstrap: str,
    topic: str,
    id_queue: queue.Queue,
    result_holder: list,
    group_prefix: str,
) -> None:
    try:
        event = _consume_reading_created_event(bootstrap, topic, id_queue, group_prefix)
        result_holder.append(event)
    except Exception as e:
        result_holder.append(e)


def _consume_reading_created_event(
    bootstrap: str,
    topic: str,
    id_queue: queue.Queue,
    group_prefix: str,
    timeout: float = 5.0,
) -> dict:
    consumer = _new_readings_kafka_consumer(bootstrap, topic, group_prefix)
    try:
        expected_id = id_queue.get(timeout=timeout)
        deadline = time.time() + timeout
        while time.time() < deadline:
            for msg in consumer.poll(timeout_ms=100).values():
                for m in msg:
                    if m.value:
                        event = json.loads(m.value.decode())
                        if event.get("id") == expected_id:
                            return event
        raise AssertionError("did not receive expected ReadingCreatedEvent before timeout")
    finally:
        consumer.close()


if __name__ == "__main__":
    sys.exit(pytest.main([os.path.dirname(os.path.abspath(__file__)), "-v", "-s"]))
