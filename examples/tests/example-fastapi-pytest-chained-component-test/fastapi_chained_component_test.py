from __future__ import annotations

import os
import sys

_TESTS_DIR = os.path.dirname(os.path.abspath(__file__))
if _TESTS_DIR not in sys.path:
    sys.path.insert(0, _TESTS_DIR)

import pytest

from arena_pytest import playbook

from playbooks import ResetReadingsDbPlaybook
from api_http import ApiClient


@playbook(ResetReadingsDbPlaybook)
def test_set_device_state_applies_requested_state(
    api_client: ApiClient,
    api_client2: ApiClient,
):
    device_id = api_client.create_device("Chained Web App Device")
    assert api_client2.get_device_state(device_id) == "OFF"

    api_client2.set_device_state(device_id, "ON")
    assert api_client.get_device_state(device_id) == "ON"

    api_client.set_device_state(device_id, "ERROR")
    assert api_client2.get_device_state(device_id) == "ERROR"

    api_client2.stop_device(device_id)


if __name__ == "__main__":
    sys.exit(
        pytest.main([os.path.dirname(os.path.abspath(__file__)), "-v", "-s"])
    )
