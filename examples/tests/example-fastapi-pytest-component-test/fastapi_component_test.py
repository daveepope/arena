from __future__ import annotations

import os
import sys
import time

_TESTS_DIR = os.path.dirname(os.path.abspath(__file__))
if _TESTS_DIR not in sys.path:
    sys.path.insert(0, _TESTS_DIR)

import pytest
import requests

from arena_pytest import ArenaBindingError, playbook

from playbooks import (
    CalibrationApiErrorPathPlaybook,
    CalibrationApiFlakyPlaybook,
    ResetValidationDbPlaybook,
)
from arena_config import CALIBRATION_VALIDATE_PATH, SMTP_UI_PORT
from api_http import ApiClient

SMTP_MESSAGES_URL = f"http://127.0.0.1:{SMTP_UI_PORT}/api/v1/messages"
MAIL_POLL_TIMEOUT_SECONDS = 10.0
MAIL_POLL_INTERVAL_SECONDS = 0.1


def _wait_device_provisioned_email(needle: str) -> None:
    deadline = time.monotonic() + MAIL_POLL_TIMEOUT_SECONDS
    while time.monotonic() < deadline:
        r = requests.get(SMTP_MESSAGES_URL, timeout=5)
        r.raise_for_status()
        if needle in r.text:
            return
        time.sleep(MAIL_POLL_INTERVAL_SECONDS)
    raise AssertionError(f"device provisioned email containing {needle!r} was not captured")


@playbook(ResetValidationDbPlaybook)
def test_create_reading_publishes_event_and_lists_via_http(
    api_client: ApiClient,
    wait_reading_created_event,
    readings_device_id: int,
):
    created_id = api_client.create_reading(
        "Readings API User", 77, "sqs happy path", readings_device_id
    )
    consumed = wait_reading_created_event(created_id)
    assert consumed["id"] == created_id
    assert consumed["user_name"] == "Readings API User"
    assert consumed["value"] == 77
    assert consumed.get("comment") == "sqs happy path"

    readings = api_client.get_readings()
    found = next((r for r in readings if r["id"] == created_id), None)
    assert found is not None
    assert found["user_name"] == "Readings API User"
    assert found["value"] == 77


@playbook(ResetValidationDbPlaybook)
def test_create_multiple_readings_are_listed(
    api_client: ApiClient, readings_device_id: int
):
    id1 = api_client.create_reading("Bending", 1, "", readings_device_id)
    id2 = api_client.create_reading(
        "joe", 2, "We're going to need a bigger ship", readings_device_id
    )
    ids = {r["id"] for r in api_client.get_readings()}
    assert id1 in ids
    assert id2 in ids


@playbook(CalibrationApiErrorPathPlaybook)
@playbook(ResetValidationDbPlaybook)
def test_post_reading_returns_500_when_calibration_outage_playbook_active(
    api_client: ApiClient, readings_device_id: int
):
    r = api_client.post_reading_raw("Outage Test User", 99, None, readings_device_id)
    assert r.status_code == 500, r.text


@playbook(ResetValidationDbPlaybook)
def test_post_reading_succeeds_after_outage_playbook_scope(
    api_client: ApiClient, readings_device_id: int
):
    recovered_id = api_client.create_reading(
        "Recovery Test User", 17, "post-outage", readings_device_id
    )
    readings = api_client.get_readings()
    found = next((r for r in readings if r["id"] == recovered_id), None)
    assert found is not None
    assert found["user_name"] == "Recovery Test User"
    assert found["value"] == 17


@playbook(ResetValidationDbPlaybook)
def test_create_reading_with_validation_db_scoped_playbook(
    api_client: ApiClient, readings_device_id: int
):
    created_id = api_client.create_reading(
        "Validation DB Scoped", 7, "mssql scope", readings_device_id
    )
    assert any(r["id"] == created_id for r in api_client.get_readings())


@playbook(CalibrationApiErrorPathPlaybook)
@playbook(ResetValidationDbPlaybook)
def test_post_reading_returns_500_under_stacked_playbooks(
    api_client: ApiClient, readings_device_id: int
):
    r = api_client.post_reading_raw("Stack Outage", 1, None, readings_device_id)
    assert r.status_code == 500, r.text


@playbook(CalibrationApiFlakyPlaybook)
@playbook(ResetValidationDbPlaybook)
def test_post_reading_succeeds_after_calibration_flaky_sequence(
    api_client: ApiClient, readings_device_id: int
):
    assert (
        api_client.post_reading_raw("Flaky 1", 1, None, readings_device_id).status_code
        == 500
    )
    assert (
        api_client.post_reading_raw("Flaky 2", 2, None, readings_device_id).status_code
        == 500
    )
    created_id = api_client.create_reading(
        "Flaky 3", 3, "recovered", readings_device_id
    )
    assert any(r["id"] == created_id for r in api_client.get_readings())


@playbook(CalibrationApiErrorPathPlaybook)
def test_http_playbook_verify_at_least_succeeds_with_traffic(
    api_client: ApiClient,
    active_http_playbook,
    readings_device_id: int,
):
    api_client.post_reading_raw("Verify At Least", 3, None, readings_device_id)
    active_http_playbook.verify_at_least("POST", CALIBRATION_VALIDATE_PATH, 1)


@playbook(CalibrationApiErrorPathPlaybook)
def test_http_playbook_verify_count_mismatch_raises(active_http_playbook):
    with pytest.raises(ArenaBindingError):
        active_http_playbook.verify("POST", CALIBRATION_VALIDATE_PATH, 1)


def test_set_device_state_applies_requested_state(
    api_client: ApiClient,
):
    device_id = api_client.create_device("Smell-O-Scope Mk II")
    assert api_client.get_device_state(device_id) == "OFF"

    api_client.set_device_state(device_id, "ON")
    assert api_client.get_device_state(device_id) == "ON"

    api_client.set_device_state(device_id, "ERROR")
    assert api_client.get_device_state(device_id) == "ERROR"

    api_client.stop_device(device_id)


def test_get_device_state_unknown_device_returns_not_found(api_client: ApiClient):
    r = api_client.get_device_state_raw(999_999_999)
    assert r.status_code == 404


def test_create_device_sends_provisioned_email_over_starttls(api_client: ApiClient):
    device_name = f"Mail Probe Device {os.urandom(4).hex()}"
    api_client.create_device(device_name)
    _wait_device_provisioned_email(device_name)


if __name__ == "__main__":
    sys.exit(
        pytest.main([os.path.dirname(os.path.abspath(__file__)), "-v", "-s"])
    )
