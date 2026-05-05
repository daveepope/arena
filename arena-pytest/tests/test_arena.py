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

from arena_pytest import active_playbooks, playbook

from readings_arena_config import (
    CALIBRATION_VALIDATE_PATH,
    DOCKER_WEB_HOST_PORT,
    EXEC_WEB_APP_PORT,
    KAFKA_CONSUMER_GROUP_LABEL,
    KAFKA_PORT,
    KAFKA_TOPIC,
)

BASE_URL_EXEC = f"http://127.0.0.1:{EXEC_WEB_APP_PORT}"
BASE_URL_DOCKER = f"http://127.0.0.1:{DOCKER_WEB_HOST_PORT}"


def _auth_headers(access_token: str) -> dict[str, str]:
    return {"Authorization": f"Bearer {access_token}"}


def test_create_reading_publishes_kafka_event_and_lists_via_http(
    arena, validation_db_playbook, oauth_access_token
):
    @playbook(validation_db_playbook)
    def _body(arena):
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
            "Exec Test User",
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
        assert consumed["user_name"] == "Exec Test User"
        assert consumed["value"] == 42
        assert consumed.get("comment") == "test comment"

        readings = _get_readings(BASE_URL_EXEC, oauth_access_token)
        found = next((r for r in readings if r["id"] == created_id), None)
        assert found is not None, "should find newly created reading"
        assert found["id"] == created_id
        assert found["user_name"] == "Exec Test User"
        assert found["value"] == 42
        assert found.get("comment") == "test comment"

    _body(arena)


def test_create_multiple_readings_are_listed(arena, validation_db_playbook, oauth_access_token):
    @playbook(validation_db_playbook)
    def _body(arena):
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

    _body(arena)


def test_post_reading_returns_500_when_calibration_api_returns_500(
    arena, calibration_identifier, oauth_access_token
):
    from arena_pytest import HttpPlaybookBuilder

    outage = (
        HttpPlaybookBuilder(calibration_identifier)
            .with_mapping(
                method="POST",
                url_path=CALIBRATION_VALIDATE_PATH,
                status=500,
                priority=1,
                expect_called=1,
            )
            .build(arena)
    )

    with outage:
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


def test_post_reading_returns_500_when_calibration_api_overridden_by_playbook(
    arena, calibration_outage_playbook, validation_db_playbook, oauth_access_token
):
    @playbook(calibration_outage_playbook, validation_db_playbook)
    def _body(arena):
        r = requests.post(
            f"{BASE_URL_EXEC}/readings",
            json={"user_name": "Decorator Outage", "value": 1, "comment": None},
            headers=_auth_headers(oauth_access_token),
            timeout=10,
        )
        assert r.status_code == 500, (
            f"expected 500 while decorator-scoped calibration outage is active, "
            f"got {r.status_code}: {r.text}"
        )

    _body(arena)

    recovered_id = _create_reading(
        BASE_URL_EXEC, "Decorator Recovery", 2, None, oauth_access_token
    )
    readings = _get_readings(BASE_URL_EXEC, oauth_access_token)
    assert any(r["id"] == recovered_id for r in readings)


def test_create_reading_with_validation_db_scoped_playbook(
    arena, validation_db_playbook, oauth_access_token
):
    @playbook(validation_db_playbook)
    def _body(arena):
        created_id = _create_reading(
            BASE_URL_EXEC,
            "Validation DB Scoped",
            7,
            "mssql scope",
            oauth_access_token,
        )
        readings = _get_readings(BASE_URL_EXEC, oauth_access_token)
        assert any(r["id"] == created_id for r in readings)

    _body(arena)


@pytest.fixture
def outage_and_db_reset(arena, calibration_outage_playbook, validation_db_playbook):
    with active_playbooks(arena, calibration_outage_playbook, validation_db_playbook):
        yield


def test_post_reading_returns_500_under_scoped_playbook_stack(arena, outage_and_db_reset, oauth_access_token):
    r = requests.post(
        f"{BASE_URL_EXEC}/readings",
        json={"user_name": "Fixture Outage", "value": 1, "comment": None},
        headers=_auth_headers(oauth_access_token),
        timeout=10,
    )
    assert r.status_code == 500, (
        f"expected 500 while fixture-scoped outage is active, "
        f"got {r.status_code}: {r.text}"
    )


def test_http_playbook_close_fails_when_call_expectation_unmet(arena, calibration_identifier):
    from arena_pytest import HttpPlaybookBuilder

    unused = (
        HttpPlaybookBuilder(calibration_identifier)
            .with_mapping(
                method="POST",
                url_path=CALIBRATION_VALIDATE_PATH,
                status=500,
                priority=1,
                expect_called=1,
            )
            .build(arena)
    )

    with pytest.raises(AssertionError, match="expected POST .* to be called exactly 1"):
        with unused:
            pass


def test_containerized_app_create_reading_publishes_kafka_event(
    docker_web_enabled,
    arena_docker,
    validation_db_playbook,
    oauth_access_token,
):
    if not docker_web_enabled:
        pytest.skip(
            "No containerized web app: OAuth issuer is loopback-only "
            "(JWT issuer cannot be reached from a bridge-network container)."
        )

    @playbook(validation_db_playbook)
    def _body(arena):
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

    _body(arena_docker)


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
    """Kafka consumer subscribed to the readings topic (one consumer per test process)."""
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
    """Thread body: poll Kafka until we see ReadingCreatedEvent for the id from id_queue."""
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
