from __future__ import annotations

import json
import os
import sys
import time
import uuid

import pytest
import pytest_asyncio

from playbooks import LocalstackSessionPurgePlaybook

from readings_ephemeral_test_runtime import ReadingsEphemeralTestRuntime, ephemeral_tcp_port

from arena_pytest import (
    ArenaLogLevel,
    ClosedArena,
    LocalstackDependencyBuilder,
    MatchBuilder,
)

LOCALSTACK_HOST_PORT = ephemeral_tcp_port()
QUEUE_NAME = "arena-events-queue"
REGION = "us-east-1"
DUMMY_CREDS = {"aws_access_key_id": "test", "aws_secret_access_key": "test"}

LOCALSTACK_ID = f"ls-{uuid.uuid4().hex[:8]}"
RUNTIME = ReadingsEphemeralTestRuntime()
NETWORK_NAME = RUNTIME.network_name("arena-network")


def _wait_for_sqs_message(
    sqs_client,
    queue_url: str,
    expected_body: str,
    timeout_s: float = 5.0,
) -> str:
    deadline = time.time() + timeout_s
    while time.time() < deadline:
        resp = sqs_client.receive_message(
            QueueUrl=queue_url,
            MaxNumberOfMessages=1,
            WaitTimeSeconds=1,
            VisibilityTimeout=5,
        )
        for msg in resp.get("Messages", []):
            body = msg["Body"]
            if body == expected_body:
                sqs_client.delete_message(
                    QueueUrl=queue_url, ReceiptHandle=msg["ReceiptHandle"]
                )
                return body
    raise AssertionError(
        f"SQS queue {queue_url} did not receive {expected_body!r} within {timeout_s}s"
    )


def _approximate_queue_depth(sqs_client, queue_url: str) -> int:
    attrs = sqs_client.get_queue_attributes(
        QueueUrl=queue_url,
        AttributeNames=["ApproximateNumberOfMessages"],
    )["Attributes"]
    return int(attrs["ApproximateNumberOfMessages"])


@pytest_asyncio.fixture(scope="module")
async def _localstack_session():
    pytest.importorskip("boto3")

    localstack = (
        LocalstackDependencyBuilder(LOCALSTACK_ID)
        .with_port(LOCALSTACK_HOST_PORT)
        .with_service("sqs")
        .with_queue(QUEUE_NAME)
        .build()
    )

    session_purge_playbook = LocalstackSessionPurgePlaybook(localstack.identifier)

    a_match = (
        MatchBuilder("localstack-e2e")
        .with_network(NETWORK_NAME)
        .add_dependency(localstack)
        .register_playbook(session_purge_playbook)
        .build()
    )

    closed = ClosedArena(
        "Localstack E2E Arena", [a_match], log_level=ArenaLogLevel.WARN
    )
    arena = None
    try:
        arena = await closed.open()
        yield arena, localstack, session_purge_playbook
    finally:
        if arena is not None:
            await arena.close()


@pytest.fixture(scope="module")
def arena(_localstack_session):
    return _localstack_session[0]


@pytest.fixture(scope="module")
def localstack_dep(_localstack_session):
    return _localstack_session[1]


@pytest.fixture(scope="module")
def session_purge_playbook(_localstack_session):
    return _localstack_session[2]


@pytest.mark.asyncio
async def test_localstack_sqs_send_receive_roundtrip(arena, localstack_dep):
    boto3 = pytest.importorskip("boto3")
    localstack = localstack_dep
    endpoint = localstack.endpoint_url("127.0.0.1")
    body = f"arena-test-{uuid.uuid4().hex[:8]}"

    sqs = boto3.client(
        "sqs", region_name=REGION, endpoint_url=endpoint, **DUMMY_CREDS
    )

    queue_url = sqs.get_queue_url(QueueName=QUEUE_NAME)["QueueUrl"]
    assert QUEUE_NAME in queue_url, (
        f"queue url should reference {QUEUE_NAME}: {queue_url}"
    )

    sqs.send_message(QueueUrl=queue_url, MessageBody=body)
    received = _wait_for_sqs_message(sqs, queue_url, body)
    assert received == body


@pytest.mark.asyncio
async def test_localstack_playbook_purges_queue(
    arena, localstack_dep, session_purge_playbook
):
    with session_purge_playbook.run(arena):
        boto3 = pytest.importorskip("boto3")
        localstack = localstack_dep
        endpoint = localstack.endpoint_url("127.0.0.1")

        sqs = boto3.client(
            "sqs", region_name=REGION, endpoint_url=endpoint, **DUMMY_CREDS
        )
        queue_url = sqs.get_queue_url(QueueName=QUEUE_NAME)["QueueUrl"]

        sqs.send_message(QueueUrl=queue_url, MessageBody=json.dumps({"n": 1}))
        sqs.send_message(QueueUrl=queue_url, MessageBody=json.dumps({"n": 2}))

        deadline = time.time() + 5
        depth = 0
        while time.time() < deadline:
            depth = _approximate_queue_depth(sqs, queue_url)
            if depth >= 2:
                break
            time.sleep(0.2)
        assert depth >= 1, (
            f"expected messages visible before playbook exit, got {depth}"
        )

    boto3 = pytest.importorskip("boto3")
    endpoint = localstack_dep.endpoint_url("127.0.0.1")
    sqs = boto3.client(
        "sqs", region_name=REGION, endpoint_url=endpoint, **DUMMY_CREDS
    )
    queue_url = sqs.get_queue_url(QueueName=QUEUE_NAME)["QueueUrl"]

    deadline = time.time() + 5
    depth = 1
    while time.time() < deadline:
        depth = _approximate_queue_depth(sqs, queue_url)
        if depth == 0:
            break
        time.sleep(0.2)
    assert depth == 0, (
        f"playbook on-exit purge should have emptied {queue_url}; got depth={depth}"
    )


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
