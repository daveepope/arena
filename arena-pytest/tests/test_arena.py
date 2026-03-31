import json
import os
import queue
import sys
import threading
import time

import requests

EXEC_WEB_APP_PORT = 3001
KAFKA_PORT = 9094
BASE_URL = f"http://127.0.0.1:{EXEC_WEB_APP_PORT}"


def test_arena_version():
    from arena_pytest import get_arena_version

    version = get_arena_version()
    assert version is not None
    assert len(version) > 0


def test_arena_shared(arena):
    assert arena.is_valid()


def _get_readings() -> list:
    r = requests.get(f"{BASE_URL}/readings", timeout=10)
    r.raise_for_status()
    return r.json()


def _create_reading(user_name: str, value: int, comment: str | None = None) -> int:
    r = requests.post(
        f"{BASE_URL}/readings",
        json={"user_name": user_name, "value": value, "comment": comment},
        timeout=10,
    )
    r.raise_for_status()
    return r.json()["id"]


def _consume_reading_created_event(bootstrap: str, topic: str, id_queue: queue.Queue, timeout: float = 5.0) -> dict:
    from kafka import KafkaConsumer

    consumer = KafkaConsumer(
        topic,
        bootstrap_servers=bootstrap,
        group_id=f"component-test-{os.getpid()}",
        auto_offset_reset="earliest",
    )
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


def test_exec_component_creates_reading_consumes_and_gets_reading(arena):
    bootstrap = f"localhost:{KAFKA_PORT}"
    id_queue = queue.Queue()
    result_holder = []

    def consume():
        try:
            event = _consume_reading_created_event(bootstrap, "readings", id_queue)
            result_holder.append(event)
        except Exception as e:
            result_holder.append(e)

    consumer_thread = threading.Thread(target=consume)
    consumer_thread.start()

    created_id = _create_reading("Exec Test User", 42, "test comment")
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

    readings = _get_readings()
    found = next((r for r in readings if r["id"] == created_id), None)
    assert found is not None, "should find newly created reading"
    assert found["id"] == created_id
    assert found["user_name"] == "Exec Test User"
    assert found["value"] == 42
    assert found.get("comment") == "test comment"


def test_multiple_readings(arena):
    id1 = _create_reading("Bending", 1, "")
    id2 = _create_reading("joe", 2, "We're going to need a bigger ship")
    readings = _get_readings()
    ids = {r["id"] for r in readings}
    assert id1 in ids
    assert id2 in ids


if __name__ == "__main__":
    import pytest
    sys.exit(pytest.main([os.path.dirname(os.path.abspath(__file__)), "-v", "-s"]))
