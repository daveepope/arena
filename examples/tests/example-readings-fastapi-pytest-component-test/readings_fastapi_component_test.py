from __future__ import annotations

import os
import sys

_TESTS_DIR = os.path.dirname(os.path.abspath(__file__))
if _TESTS_DIR not in sys.path:
    sys.path.insert(0, _TESTS_DIR)

import pytest
import requests

from arena_pytest import playbook

from readings_playbooks import LocalstackSessionPlaybook, ValidationDbPlaybook


@playbook(ValidationDbPlaybook)
@playbook(LocalstackSessionPlaybook)
@pytest.mark.asyncio
async def test_fastapi_readings_happy_path_sqs_event_and_list(readings_fastapi_ctx):
    boto3 = pytest.importorskip("boto3")
    ctx = readings_fastapi_ctx

    sqs = boto3.client(
        "sqs",
        region_name=ctx.region,
        endpoint_url=ctx.localstack_endpoint,
        **ctx.dummy_aws_creds,
    )
    queue_url = sqs.get_queue_url(QueueName=ctx.queue_name)["QueueUrl"]

    s = requests.Session()
    s.verify = ctx.oauth_ca_path
    headers = {"Authorization": f"Bearer {ctx.access_token}"}
    body = {
        "user_name": "Readings API User",
        "value": 77,
        "comment": "sqs happy path",
    }
    r = s.post(
        f"{ctx.web_base}/readings",
        json=body,
        headers=headers,
        timeout=60,
    )
    assert r.status_code == 200, r.text
    created = r.json()
    assert created.get("valid") is True
    rid = int(created["id"])

    detail = ctx.wait_sqs_reading_created(sqs, queue_url, rid)
    assert detail["id"] == rid
    assert detail["user_name"] == body["user_name"]
    assert detail["value"] == body["value"]
    assert detail.get("comment") == body["comment"]

    g = s.get(f"{ctx.web_base}/readings", headers=headers, timeout=60)
    assert g.status_code == 200, g.text
    rows = g.json()
    found = next((x for x in rows if int(x["id"]) == rid), None)
    assert found is not None
    assert found["user_name"] == body["user_name"]
    assert found["value"] == body["value"]


if __name__ == "__main__":
    sys.exit(
        pytest.main(
            [
                os.path.dirname(os.path.abspath(__file__)),
                "-v",
                "-s",
            ]
        )
    )
