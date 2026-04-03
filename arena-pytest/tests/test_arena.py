"""Integration tests: shared Postgres + Kafka + arena with exec binary and Docker web app (see conftest)."""

import json
import os
import queue
import sys
import threading
import time
from typing import Any

import pytest
import requests

EXEC_WEB_APP_PORT = 3001
DOCKER_WEB_HOST_PORT = 3002
KAFKA_PORT = 9094
KAFKA_TOPIC = "readings"

BASE_URL_EXEC = f"http://127.0.0.1:{EXEC_WEB_APP_PORT}"
BASE_URL_DOCKER = f"http://127.0.0.1:{DOCKER_WEB_HOST_PORT}"

# Bazel runfiles are sparse: Dockerfile / full build context is not available for docker build.
_SKIP_DOCKER_WEB_WHEN_RUNFILES = pytest.mark.skipif(
    bool(os.environ.get("RUNFILES_DIR")),
    reason="Docker web app build context not in runfiles; run pytest from repo root for full stack.",
)


def test_exec_arena_version():
    from arena_pytest import get_arena_version

    version = get_arena_version()
    assert version is not None
    assert len(version) > 0


def test_exec_reading_flow_kafka_and_http(arena):
    bootstrap = f"localhost:{KAFKA_PORT}"
    id_queue: queue.Queue[int] = queue.Queue()
    result_holder: list[Any] = []

    consumer_thread = threading.Thread(
        target=_run_reading_created_consumer,
        args=(bootstrap, KAFKA_TOPIC, id_queue, result_holder, "exec"),
    )
    consumer_thread.start()

    created_id = _create_reading(BASE_URL_EXEC, "Exec Test User", 42, "test comment")
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

    readings = _get_readings(BASE_URL_EXEC)
    found = next((r for r in readings if r["id"] == created_id), None)
    assert found is not None, "should find newly created reading"
    assert found["id"] == created_id
    assert found["user_name"] == "Exec Test User"
    assert found["value"] == 42
    assert found.get("comment") == "test comment"


def test_exec_multiple_readings(arena):
    id1 = _create_reading(BASE_URL_EXEC, "Bending", 1, "")
    id2 = _create_reading(BASE_URL_EXEC, "joe", 2, "We're going to need a bigger ship")
    readings = _get_readings(BASE_URL_EXEC)
    ids = {r["id"] for r in readings}
    assert id1 in ids
    assert id2 in ids


@_SKIP_DOCKER_WEB_WHEN_RUNFILES
def test_docker_reading_flow_kafka_and_http(arena_docker):
    bootstrap = f"localhost:{KAFKA_PORT}"
    id_queue: queue.Queue[int] = queue.Queue()
    result_holder: list[Any] = []

    consumer_thread = threading.Thread(
        target=_run_reading_created_consumer,
        args=(bootstrap, KAFKA_TOPIC, id_queue, result_holder, "docker"),
    )
    consumer_thread.start()

    created_id = _create_reading(BASE_URL_DOCKER, "Docker Test User", 42, "test comment")
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

    readings = _get_readings(BASE_URL_DOCKER)
    found = next((r for r in readings if r["id"] == created_id), None)
    assert found is not None, "should find newly created reading"
    assert found["id"] == created_id
    assert found["user_name"] == "Docker Test User"
    assert found["value"] == 42
    assert found.get("comment") == "test comment"


def _get_readings(base_url: str) -> list:
    r = requests.get(f"{base_url}/readings", timeout=10)
    r.raise_for_status()
    return r.json()


def _create_reading(
    base_url: str,
    user_name: str,
    value: int,
    comment: str | None = None,
) -> int:
    r = requests.post(
        f"{base_url}/readings",
        json={"user_name": user_name, "value": value, "comment": comment},
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
        group_id=f"{group_prefix}-component-test-{os.getpid()}",
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
