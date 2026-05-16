from __future__ import annotations

import json
import os
import sys
import time
import uuid

import pytest
import pytest_asyncio

from arena_pytest import (
    ArenaLogLevel,
    ClosedArena,
    EventRuleSpec,
    EventRuleTarget,
    LambdaSpec,
    LambdaTarget,
    LocalstackDependencyBuilder,
    ManagedLocalstackPlaybook,
    MatchBuilder,
    SqsQueueTarget,
)
from arena_pytest.playbook import active_playbooks

LOCALSTACK_HOST_PORT = 4567
LOCALSTACK_NETWORK = "arena-pytest-localstack-network"
QUEUE_NAME = "arena-events-queue"
LAMBDA_NAME = "arena-echo-lambda"
EVENT_BUS_NAME = "arena-event-bus"
EVENT_RULE_NAME = "arena-route-all"
EVENT_SOURCE = "arena.test"
REGION = "us-east-1"
DUMMY_CREDS = {"aws_access_key_id": "test", "aws_secret_access_key": "test"}

LOCALSTACK_ID = f"ls-{uuid.uuid4().hex[:8]}"
MANAGED_LOCALSTACK_PLAYBOOK_ID = "localstack-session-purge"


def _write_lambda_source(base_dir) -> str:
    src = base_dir / "lambda_src"
    src.mkdir()
    (src / "handler.py").write_text(
        "def handler(event, context):\n"
        "    return {'statusCode': 200, 'body': 'ok', 'received': event}\n"
    )
    return str(src)


def _wait_for_sqs_message(
    sqs_client,
    queue_url: str,
    expected_detail_type: str,
    timeout_s: float = 30.0,
) -> dict:
    deadline = time.time() + timeout_s
    while time.time() < deadline:
        resp = sqs_client.receive_message(
            QueueUrl=queue_url,
            MaxNumberOfMessages=1,
            WaitTimeSeconds=1,
            VisibilityTimeout=5,
        )
        for msg in resp.get("Messages", []):
            body = json.loads(msg["Body"])
            if body.get("detail-type") == expected_detail_type:
                sqs_client.delete_message(
                    QueueUrl=queue_url, ReceiptHandle=msg["ReceiptHandle"]
                )
                return body
    raise AssertionError(
        f"SQS queue {queue_url} did not receive a message of type "
        f"{expected_detail_type!r} within {timeout_s}s"
    )


def _approximate_queue_depth(sqs_client, queue_url: str) -> int:
    attrs = sqs_client.get_queue_attributes(
        QueueUrl=queue_url,
        AttributeNames=["ApproximateNumberOfMessages"],
    )["Attributes"]
    return int(attrs["ApproximateNumberOfMessages"])


@pytest_asyncio.fixture(scope="module")
async def _localstack_session(tmp_path_factory):
    pytest.importorskip("boto3")

    base = tmp_path_factory.mktemp("localstack-arena")
    lambda_src = _write_lambda_source(base)

    localstack = (
        LocalstackDependencyBuilder(LOCALSTACK_ID)
        .with_port(LOCALSTACK_HOST_PORT)
        .with_services(["sqs", "lambda", "events"])
        .with_queue(QUEUE_NAME)
        .with_lambda(
            LambdaSpec(
                name=LAMBDA_NAME,
                runtime="python3.12",
                handler="handler.handler",
                source_dir=lambda_src,
            )
        )
        .with_event_bus(EVENT_BUS_NAME)
        .with_event_rule(
            EventRuleSpec(
                name=EVENT_RULE_NAME,
                event_bus=EVENT_BUS_NAME,
                event_pattern=json.dumps({"source": [EVENT_SOURCE]}),
                targets=[
                    EventRuleTarget(
                        target_id="target-queue",
                        kind=SqsQueueTarget(queue_name=QUEUE_NAME),
                    ),
                    EventRuleTarget(
                        target_id="target-lambda",
                        kind=LambdaTarget(function_name=LAMBDA_NAME),
                    ),
                ],
            )
        )
        .build()
    )

    session_purge_playbook = ManagedLocalstackPlaybook(
        MANAGED_LOCALSTACK_PLAYBOOK_ID,
        localstack.identifier,
    )

    a_match = (
        MatchBuilder("localstack-e2e")
        .with_network(LOCALSTACK_NETWORK)
        .add_dependency(localstack)
        .register_playbook(session_purge_playbook)
        .build()
    )

    closed = ClosedArena(
        "Localstack E2E Arena", [a_match], log_level=ArenaLogLevel.DEBUG
    )
    arena = await closed.open()
    try:
        yield arena, localstack, session_purge_playbook
    finally:
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
async def test_localstack_full_stack_end_to_end(arena, localstack_dep):
    boto3 = pytest.importorskip("boto3")
    localstack = localstack_dep
    endpoint = localstack.endpoint_url("127.0.0.1")
    run_id = uuid.uuid4().hex[:8]

    sqs = boto3.client(
        "sqs", region_name=REGION, endpoint_url=endpoint, **DUMMY_CREDS
    )
    lam = boto3.client(
        "lambda", region_name=REGION, endpoint_url=endpoint, **DUMMY_CREDS
    )
    events = boto3.client(
        "events", region_name=REGION, endpoint_url=endpoint, **DUMMY_CREDS
    )

    queue_url = sqs.get_queue_url(QueueName=QUEUE_NAME)["QueueUrl"]
    assert QUEUE_NAME in queue_url, (
        f"queue url should reference {QUEUE_NAME}: {queue_url}"
    )

    fn = lam.get_function(FunctionName=LAMBDA_NAME)
    assert fn["Configuration"]["FunctionName"] == LAMBDA_NAME
    assert fn["Configuration"]["Runtime"] == "python3.12"

    rule = events.describe_rule(Name=EVENT_RULE_NAME, EventBusName=EVENT_BUS_NAME)
    assert rule["Name"] == EVENT_RULE_NAME
    assert json.loads(rule["EventPattern"]) == {"source": [EVENT_SOURCE]}

    targets = events.list_targets_by_rule(
        Rule=EVENT_RULE_NAME, EventBusName=EVENT_BUS_NAME
    )["Targets"]
    target_ids = {t["Id"] for t in targets}
    assert target_ids == {"target-queue", "target-lambda"}, (
        f"expected both SQS and Lambda targets, got {target_ids}"
    )

    target_arns = {t["Arn"] for t in targets}
    assert localstack.queue_arn(QUEUE_NAME) in target_arns
    assert localstack.lambda_arn(LAMBDA_NAME) in target_arns

    detail_type = f"arena-test-{run_id}"
    events.put_events(
        Entries=[
            {
                "Source": EVENT_SOURCE,
                "DetailType": detail_type,
                "EventBusName": EVENT_BUS_NAME,
                "Detail": json.dumps({"run_id": run_id, "value": 42}),
            }
        ]
    )

    received = _wait_for_sqs_message(sqs, queue_url, detail_type)
    assert received["source"] == EVENT_SOURCE
    assert received["detail-type"] == detail_type
    assert received["detail"]["run_id"] == run_id
    assert received["detail"]["value"] == 42


@pytest.mark.asyncio
async def test_localstack_playbook_purges_queue(
    arena, localstack_dep, session_purge_playbook
):
    with active_playbooks(arena, session_purge_playbook):
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


@pytest.mark.asyncio
async def test_localstack_queue_is_empty_after_playbook(arena, localstack_dep):
    boto3 = pytest.importorskip("boto3")
    localstack = localstack_dep
    endpoint = localstack.endpoint_url("127.0.0.1")

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
    sys.exit(pytest.main([os.path.dirname(os.path.abspath(__file__)), "-v", "-s"]))
