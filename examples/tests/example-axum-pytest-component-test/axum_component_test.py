import os
import sys

_TESTS_DIR = os.path.dirname(os.path.abspath(__file__))
if _TESTS_DIR not in sys.path:
    sys.path.insert(0, _TESTS_DIR)

import pytest

from arena_pytest import ArenaBindingError, playbook

from playbooks import (
    CalibrationApiErrorPathPlaybook,
    CalibrationApiFlakyPlaybook,
    ResetValidationDbPlaybook,
)
from arena_config import CALIBRATION_VALIDATE_PATH
from api_http import ApiClient


@playbook(ResetValidationDbPlaybook)
def test_create_reading_publishes_event_and_lists_via_http(
    api_client: ApiClient,
    wait_reading_created_event,
):
    consumed = wait_reading_created_event(
        lambda: api_client.create_reading("Readings API User", 77, "kafka happy path")
    )
    created_id = consumed["id"]
    assert consumed["id"] == created_id
    assert consumed["user_name"] == "Readings API User"
    assert consumed["value"] == 77
    assert consumed.get("comment") == "kafka happy path"

    readings = api_client.get_readings()
    found = next((r for r in readings if r["id"] == created_id), None)
    assert found is not None
    assert found["user_name"] == "Readings API User"
    assert found["value"] == 77


@playbook(ResetValidationDbPlaybook)
def test_create_multiple_readings_are_listed(api_client: ApiClient):
    id1 = api_client.create_reading("Bending", 1, "")
    id2 = api_client.create_reading("joe", 2, "We're going to need a bigger ship")
    ids = {r["id"] for r in api_client.get_readings()}
    assert id1 in ids
    assert id2 in ids


@playbook(CalibrationApiErrorPathPlaybook)
@playbook(ResetValidationDbPlaybook)
def test_post_reading_returns_500_when_calibration_outage_playbook_active(
    api_client: ApiClient,
):
    r = api_client.post_reading_raw("Outage Test User", 99, None)
    assert r.status_code == 500, r.text


@playbook(ResetValidationDbPlaybook)
def test_post_reading_succeeds_after_outage_playbook_scope(api_client: ApiClient):
    recovered_id = api_client.create_reading("Recovery Test User", 17, "post-outage")
    readings = api_client.get_readings()
    found = next((r for r in readings if r["id"] == recovered_id), None)
    assert found is not None
    assert found["user_name"] == "Recovery Test User"
    assert found["value"] == 17


@playbook(ResetValidationDbPlaybook)
def test_create_reading_with_validation_db_scoped_playbook(api_client: ApiClient):
    created_id = api_client.create_reading("Validation DB Scoped", 7, "mssql scope")
    assert any(r["id"] == created_id for r in api_client.get_readings())


@playbook(CalibrationApiErrorPathPlaybook)
@playbook(ResetValidationDbPlaybook)
def test_post_reading_returns_500_under_stacked_playbooks(api_client: ApiClient):
    r = api_client.post_reading_raw("Stack Outage", 1, None)
    assert r.status_code == 500, r.text


@playbook(CalibrationApiFlakyPlaybook)
@playbook(ResetValidationDbPlaybook)
def test_post_reading_succeeds_after_calibration_flaky_sequence(api_client: ApiClient):
    assert api_client.post_reading_raw("Flaky 1", 1, None).status_code == 500
    assert api_client.post_reading_raw("Flaky 2", 2, None).status_code == 500
    created_id = api_client.create_reading("Flaky 3", 3, "recovered")
    assert any(r["id"] == created_id for r in api_client.get_readings())


@playbook(CalibrationApiErrorPathPlaybook)
def test_http_playbook_verify_at_least_succeeds_with_traffic(
    api_client: ApiClient,
    active_http_playbook,
):
    api_client.post_reading_raw("Verify At Least", 3, None)
    active_http_playbook.verify_at_least("POST", CALIBRATION_VALIDATE_PATH, 1)


@playbook(CalibrationApiErrorPathPlaybook)
def test_http_playbook_verify_count_mismatch_raises(active_http_playbook):
    with pytest.raises(ArenaBindingError):
        active_http_playbook.verify("POST", CALIBRATION_VALIDATE_PATH, 1)


if __name__ == "__main__":
    sys.exit(
        pytest.main(
            [
                os.path.dirname(os.path.abspath(__file__)),
                "-v",
                "-s",
                "-o",
                "asyncio_mode=auto",
                "-o",
                "asyncio_default_fixture_loop_scope=session",
            ]
        )
    )
