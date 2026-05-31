from __future__ import annotations

import json
import logging
import os
import sys
import tempfile
import time
import uuid
from dataclasses import dataclass

_TESTS_DIR = os.path.dirname(os.path.abspath(__file__))
if _TESTS_DIR not in sys.path:
    sys.path.insert(0, _TESTS_DIR)

import pytest


def _arena_plugin_already_registered_via_entry_point() -> bool:
    try:
        from importlib.metadata import entry_points
        eps = entry_points(group="pytest11")
    except Exception:
        return False
    return any(getattr(ep, "value", "") == "arena_pytest.arena" for ep in eps)


if not _arena_plugin_already_registered_via_entry_point():
    pytest_plugins = ("arena_pytest.arena",)

import pytest_asyncio
import requests

from arena_pytest import (
    ArenaLogLevel,
    ClosedArena,
    EventRuleSpec,
    EventRuleTarget,
    ExecutableComponentBuilder,
    HttpDependencyBuilder,
    HttpReadinessCheck,
    LocalstackDependencyBuilder,
    MatchBuilder,
    MssqlDependencyBuilder,
    OauthDependencyBuilder,
    PostgresDependencyBuilder,
    SqsQueueTarget,
    oauth_loopback_tls_pem_pair,
)

from readings_ephemeral_test_runtime import RUNTIME

from readings_playbooks import (
    CalibrationDefaultPlaybook,
    LocalstackSessionPlaybook,
    ValidationDbPlaybook,
)

WEB_APP_PORT = RUNTIME.exec_web_app_port
POSTGRES_PORT = RUNTIME.postgres_port
MSSQL_PORT = RUNTIME.mssql_port
CALIBRATION_HOST_PORT = RUNTIME.calibration_host_port
LOCALSTACK_HOST_PORT = RUNTIME.localstack_host_port
OAUTH_PORT = RUNTIME.oauth_port
OAUTH_ISSUER = RUNTIME.oauth_issuer
POSTGRES_DB_NAME = "readings_db"
POSTGRES_DB_USER = "readings_user"
POSTGRES_DB_PASS = "readings_password"
MSSQL_DB_NAME = "validationDb"
MSSQL_DB_USER = "sa"
MSSQL_DB_PASS = "yourStrong(!)Password"
NETWORK_NAME = RUNTIME.network_name("arena-readings-api-network")
EVENT_BUS_NAME = "readings-api-events"
EVENT_SOURCE = "readings.api"
QUEUE_NAME = "readings-api-events-q"
EVENT_RULE_NAME = "readings-api-rule"
DUMMY_CREDS = {"aws_access_key_id": "test", "aws_secret_access_key": "test"}
REGION = "us-east-1"

_LOG = logging.getLogger(__name__)


def _find_resource_file(filename: str) -> str:
    try:
        from bazel_tools.tools.python.runfiles import runfiles

        r = runfiles.Create()
        for rel in (
            f"arena/examples/resources/{filename}",
            f"_main/examples/resources/{filename}",
        ):
            p = r.Rlocation(rel)
            if p and os.path.isfile(p):
                return p
    except ImportError:
        pass
    runfiles_dir = os.environ.get("RUNFILES_DIR")
    if runfiles_dir:
        for base in ("_main", "arena", ""):
            p = os.path.join(runfiles_dir, base, "examples", "resources", filename)
            if os.path.isfile(p):
                return p
    root = os.path.dirname(
        os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    )
    p = os.path.join(root, "resources", filename)
    return p if os.path.isfile(p) else ""


def _find_fastapi_executable() -> str:
    runfiles_dir = os.environ.get("RUNFILES_DIR")
    if runfiles_dir:
        for base in ("_main", "arena", ""):
            p = os.path.join(
                runfiles_dir, base, "examples", "example-readings-fastapi-web-app"
            )
            if os.path.isfile(p):
                return p
    return ""


def _fetch_access_token(ca_path: str, issuer: str) -> str:
    s = requests.Session()
    s.verify = ca_path
    disc = s.get(f"{issuer}/.well-known/oauth-authorization-server", timeout=60)
    disc.raise_for_status()
    token_url = disc.json()["token_endpoint"]
    tok = s.post(
        token_url,
        data={
            "grant_type": "client_credentials",
            "client_id": "arena-examples",
            "scope": "readings",
        },
        timeout=60,
    )
    tok.raise_for_status()
    return str(tok.json()["access_token"])


@dataclass(frozen=True)
class ReadingsFastapiCtx:
    arena: object
    oauth_ca_path: str
    access_token: str
    web_base: str
    localstack_endpoint: str
    queue_name: str
    region: str
    dummy_aws_creds: dict[str, str]

    def wait_sqs_reading_created(
        self,
        sqs_client,
        queue_url: str,
        expected_id: int,
        timeout_s: float = 45.0,
    ) -> dict:
        deadline = time.time() + timeout_s
        while time.time() < deadline:
            resp = sqs_client.receive_message(
                QueueUrl=queue_url,
                MaxNumberOfMessages=1,
                WaitTimeSeconds=2,
                VisibilityTimeout=10,
            )
            for msg in resp.get("Messages", []):
                body = json.loads(msg["Body"])
                if body.get("detail-type") != "ReadingCreated":
                    sqs_client.delete_message(
                        QueueUrl=queue_url, ReceiptHandle=msg["ReceiptHandle"]
                    )
                    continue
                detail = body.get("detail")
                if isinstance(detail, str):
                    detail = json.loads(detail)
                if int(detail.get("id", -1)) == expected_id:
                    sqs_client.delete_message(
                        QueueUrl=queue_url, ReceiptHandle=msg["ReceiptHandle"]
                    )
                    return detail
                sqs_client.delete_message(
                    QueueUrl=queue_url, ReceiptHandle=msg["ReceiptHandle"]
                )
        raise AssertionError(
            f"SQS did not receive ReadingCreated for id={expected_id} within {timeout_s}s"
        )


_OAUTH_CA_FILE = ""


@pytest.fixture(scope="session")
def closed_arena() -> ClosedArena:
    global _OAUTH_CA_FILE
    pytest.importorskip("boto3")
    ca_pem, server_key_pem = oauth_loopback_tls_pem_pair()

    fd, oauth_ca_file = tempfile.mkstemp(prefix="readings-api-oauth-", suffix=".pem")
    with os.fdopen(fd, "w", encoding="utf-8") as f:
        f.write(ca_pem)
    _OAUTH_CA_FILE = oauth_ca_file

    schema_path = _find_resource_file("instrument_reading_db_schema.sql")
    if not schema_path:
        pytest.fail("instrument_reading_db_schema.sql not found")
    mssql_schema_path = _find_resource_file("validation_db_schema.sql")
    if not mssql_schema_path:
        pytest.fail("validation_db_schema.sql not found")
    startup_sql = [open(schema_path, encoding="utf-8").read()]
    mssql_startup_sql = [open(mssql_schema_path, encoding="utf-8").read()]

    oauth = (
        OauthDependencyBuilder("readings-api-oauth")
        .with_port(OAUTH_PORT)
        .with_listen_ip("0.0.0.0")
        .with_server_tls_pem(ca_pem, server_key_pem)
        .with_metadata_base_url(OAUTH_ISSUER)
        .build()
    )

    postgres = (
        PostgresDependencyBuilder("readings-api-postgres")
        .with_image("14.20-trixie")
        .with_port(POSTGRES_PORT)
        .with_database_name(POSTGRES_DB_NAME)
        .with_database_username(POSTGRES_DB_USER)
        .with_database_password(POSTGRES_DB_PASS)
        .with_startup_sql_scripts(startup_sql)
        .build()
    )

    mssql = (
        MssqlDependencyBuilder("readings-api-mssql")
        .with_port(MSSQL_PORT)
        .with_database_name(MSSQL_DB_NAME)
        .with_database_username(MSSQL_DB_USER)
        .with_database_password(MSSQL_DB_PASS)
        .with_startup_sql_scripts(mssql_startup_sql)
        .build()
    )

    calibration = (
        HttpDependencyBuilder("readings-api-calibration")
        .with_port(CALIBRATION_HOST_PORT)
        .build()
    )

    ls_id = f"ls-readings-api-{uuid.uuid4().hex[:8]}"
    localstack = (
        LocalstackDependencyBuilder(ls_id)
        .with_port(LOCALSTACK_HOST_PORT)
        .with_services(["sqs", "events"])
        .with_queue(QUEUE_NAME)
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
                ],
            )
        )
        .build()
    )

    exe = _find_fastapi_executable()
    if not exe:
        pytest.fail(
            "example-readings-fastapi-web-app not found (build //examples:example-readings-fastapi-web-app)"
        )

    mssql_cs = (
        f"Server=tcp:localhost,{MSSQL_PORT};Database={MSSQL_DB_NAME};"
        f"User Id={MSSQL_DB_USER};Password={MSSQL_DB_PASS};TrustServerCertificate=True;"
    )
    pg_cs = (
        f"host=localhost port={POSTGRES_PORT} user={POSTGRES_DB_USER} "
        f"password={POSTGRES_DB_PASS} dbname={POSTGRES_DB_NAME}"
    )
    ls_ep = f"http://127.0.0.1:{LOCALSTACK_HOST_PORT}"

    fastapi_component = (
        ExecutableComponentBuilder("readings-api-web-app")
        .with_executable_path(exe)
        .with_env_var("WEB_APP_PORT", str(WEB_APP_PORT))
        .with_env_var("POSTGRES_CONNECTION_STRING", pg_cs)
        .with_env_var("CALIBRATION_URL", f"http://127.0.0.1:{CALIBRATION_HOST_PORT}")
        .with_env_var("MSSQL_CONNECTION_STRING", mssql_cs)
        .with_env_var("OAUTH_ISSUER_URL", OAUTH_ISSUER)
        .with_env_var("OAUTH_TLS_CA_FILE", str(oauth_ca_file))
        .with_env_var("OAUTH_REQUIRED_ACCESS_TOKEN_SCOPES", "readings")
        .with_env_var("AWS_ENDPOINT_URL", ls_ep)
        .with_env_var("AWS_DEFAULT_REGION", REGION)
        .with_env_var("AWS_ACCESS_KEY_ID", DUMMY_CREDS["aws_access_key_id"])
        .with_env_var("AWS_SECRET_ACCESS_KEY", DUMMY_CREDS["aws_secret_access_key"])
        .with_env_var("EVENT_BUS_NAME", EVENT_BUS_NAME)
        .with_env_var("EVENT_SOURCE", EVENT_SOURCE)
        .with_readiness_check(
            HttpReadinessCheck.create(), f"http://127.0.0.1:{WEB_APP_PORT}/health"
        )
        .build()
    )

    a_match = (
        MatchBuilder("readings-api-happy-path")
        .with_network(NETWORK_NAME)
        .add_dependency(oauth)
        .add_dependency(postgres)
        .add_dependency(mssql)
        .add_dependency(calibration)
        .add_dependency(localstack)
        .add_component(fastapi_component)
        .register_playbook(
            CalibrationDefaultPlaybook(calibration.identifier),
            exec_on_dependency_start=True,
        )
        .register_playbook(LocalstackSessionPlaybook(localstack.identifier))
        .register_playbook(ValidationDbPlaybook(mssql.identifier))
        .build()
    )

    return ClosedArena(
        "readings-api-arena",
        [a_match],
        log_level=ArenaLogLevel.DEBUG,
        logger=_LOG,
        log_component_ids=("readings-api-web-app",),
        log_dependency_ids=(
            oauth.identifier,
            postgres.identifier,
            mssql.identifier,
            calibration.identifier,
            localstack.identifier,
        ),
    )


@pytest_asyncio.fixture(scope="session")
async def readings_fastapi_ctx(arena) -> ReadingsFastapiCtx:
    token = _fetch_access_token(_OAUTH_CA_FILE, OAUTH_ISSUER)
    ls_ep = f"http://127.0.0.1:{LOCALSTACK_HOST_PORT}"
    return ReadingsFastapiCtx(
        arena=arena,
        oauth_ca_path=_OAUTH_CA_FILE,
        access_token=token,
        web_base=f"http://127.0.0.1:{WEB_APP_PORT}",
        localstack_endpoint=ls_ep,
        queue_name=QUEUE_NAME,
        region=REGION,
        dummy_aws_creds=DUMMY_CREDS,
    )
