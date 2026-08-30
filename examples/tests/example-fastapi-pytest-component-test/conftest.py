from __future__ import annotations

import json
import logging
import os
import sys
import tempfile
import time
import uuid
from datetime import timedelta

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

import requests

from arena_pytest import (
    ArenaLogLevel,
    ClosedArena,
    Custom,
    EventRuleSpec,
    EventRuleTarget,
    ExecutableComponentBuilder,
    HttpDependencyBuilder,
    HttpReadinessCheck,
    LocalstackDependencyBuilder,
    MatchBuilder,
    MssqlDependencyBuilder,
    OauthDependencyBuilder,
    OracleDependencyBuilder,
    PostgresDependencyBuilder,
    SmtpDependencyBuilder,
    SqsQueueTarget,
    TemporalDependencyBuilder,
    oauth_loopback_tls_pem_pair,
    oauth_signer_fixture,
)

from arena_config import (
    CALIBRATION_CONTAINER_NAME,
    CALIBRATION_HOST_PORT,
    CLOSED_ARENA_NAME,
    COMPONENT_NAME_EXECUTABLE,
    DEP_NAME_CALIBRATION_HTTP,
    DEP_NAME_MSSQL,
    DEP_NAME_OAUTH,
    DEP_NAME_ORACLE,
    DEP_NAME_POSTGRES,
    DEP_NAME_SMTP,
    DEP_NAME_TEMPORAL,
    EXEC_WEB_APP_PORT,
    LOCALSTACK_HOST_PORT,
    MATCH_NAME,
    MSSQL_CONNECTION_STRING_LOCAL,
    MSSQL_CONTAINER_NAME,
    MSSQL_DB_NAME,
    MSSQL_DB_PASS,
    MSSQL_DB_USER,
    MSSQL_PORT,
    OAUTH_ISSUER,
    OAUTH_PORT,
    ORACLE_ADMIN_PASS,
    ORACLE_CONNECTION_STRING_LOCAL,
    ORACLE_CONTAINER_NAME,
    ORACLE_DB_PASS,
    ORACLE_DB_USER,
    ORACLE_PORT,
    POSTGRES_CONTAINER_NAME,
    POSTGRES_DB_NAME,
    POSTGRES_DB_PASS,
    POSTGRES_DB_USER,
    POSTGRES_PORT,
    SMTP_CONTAINER_NAME,
    SMTP_HOST_PORT,
    SMTP_UI_PORT,
    TEMPORAL_CONTAINER_NAME,
    TEMPORAL_GRPC_PORT,
    TEMPORAL_UI_PORT,
)

from api_http import ApiClient
from oauth_claims import claims_with_scope
from playbook_fixtures import active_http_playbook
from playbooks import (
    CalibrationApiHappyPathPlaybook,
    CalibrationApiErrorPathPlaybook,
    CalibrationApiFlakyPlaybook,
    EventsPurgePlaybook,
    ResetValidationDbPlaybook,
    ResetWeatherDbPlaybook,
    SeedValidationReadingPlaybook,
)
WEB_APP_PORT = EXEC_WEB_APP_PORT
EVENT_BUS_NAME = "example-api-events"
EVENT_SOURCE = "readings.api"
QUEUE_NAME = "example-api-events-q"
EVENT_RULE_NAME = "example-api-rule"
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


_OAUTH_CA_FILE = ""
_OAUTH = None


def _wait_sqs_reading_created(
    sqs_client,
    queue_url: str,
    expected_id: int,
    timeout_s: float = 10.0,
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


@pytest.fixture(scope="session")
def closed_arena() -> ClosedArena:
    global _OAUTH_CA_FILE, _OAUTH
    pytest.importorskip("boto3")
    ca_pem, server_key_pem = oauth_loopback_tls_pem_pair()

    fd, oauth_ca_file = tempfile.mkstemp(prefix="example-api-oauth-", suffix=".pem")
    with os.fdopen(fd, "w", encoding="utf-8") as f:
        f.write(ca_pem)
    _OAUTH_CA_FILE = oauth_ca_file

    schema_path = _find_resource_file("instrument_reading_db_schema.sql")
    if not schema_path:
        pytest.fail("instrument_reading_db_schema.sql not found")
    mssql_schema_path = _find_resource_file("validation_db_schema.sql")
    if not mssql_schema_path:
        pytest.fail("validation_db_schema.sql not found")
    oracle_schema_path = _find_resource_file("weather_db_schema.sql")
    if not oracle_schema_path:
        pytest.fail("weather_db_schema.sql not found")
    startup_sql = [open(schema_path, encoding="utf-8").read()]
    mssql_startup_sql = [open(mssql_schema_path, encoding="utf-8").read()]
    oracle_startup_sql = [open(oracle_schema_path, encoding="utf-8").read()]

    oauth = (
        OauthDependencyBuilder(DEP_NAME_OAUTH)
        .with_port(OAUTH_PORT)
        .with_listen_ip("0.0.0.0")
        .with_server_tls_pem(ca_pem, server_key_pem)
        .with_metadata_base_url(OAUTH_ISSUER)
        .build()
    )
    _OAUTH = oauth

    postgres = (
        PostgresDependencyBuilder(DEP_NAME_POSTGRES)
        .with_container_name(POSTGRES_CONTAINER_NAME)
        .with_image("14.20-trixie")
        .with_port(POSTGRES_PORT)
        .with_database_name(POSTGRES_DB_NAME)
        .with_database_username(POSTGRES_DB_USER)
        .with_database_password(POSTGRES_DB_PASS)
        .with_startup_sql_scripts(startup_sql)
        .build()
    )

    mssql = (
        MssqlDependencyBuilder(DEP_NAME_MSSQL)
        .with_container_name(MSSQL_CONTAINER_NAME)
        .with_port(MSSQL_PORT)
        .with_database_name(MSSQL_DB_NAME)
        .with_database_username(MSSQL_DB_USER)
        .with_database_password(MSSQL_DB_PASS)
        .with_startup_sql_scripts(mssql_startup_sql)
        .build()
    )

    oracle = (
        OracleDependencyBuilder(DEP_NAME_ORACLE)
        .with_container_name(ORACLE_CONTAINER_NAME)
        .with_port(ORACLE_PORT)
        .with_database_username(ORACLE_DB_USER)
        .with_database_password(ORACLE_DB_PASS)
        .with_admin_password(ORACLE_ADMIN_PASS)
        .with_startup_sql_scripts(oracle_startup_sql)
        # Oracle container start times are inconsistent across CI runners, so a longer timeout is required.
        .with_sql_readiness_timeout(timedelta(minutes=2))
        .build()
    )

    calibration = (
        HttpDependencyBuilder(DEP_NAME_CALIBRATION_HTTP)
        .with_container_name(CALIBRATION_CONTAINER_NAME)
        .with_port(CALIBRATION_HOST_PORT)
        .build()
    )

    temporal = (
        TemporalDependencyBuilder(DEP_NAME_TEMPORAL)
        .with_container_name(TEMPORAL_CONTAINER_NAME)
        .with_image("1.8.0")
        .with_port(TEMPORAL_GRPC_PORT)
        .with_ui_port(TEMPORAL_UI_PORT)
        .build()
    )

    smtp = (
        SmtpDependencyBuilder(DEP_NAME_SMTP)
        .with_container_name(SMTP_CONTAINER_NAME)
        .with_port(SMTP_HOST_PORT)
        .with_ui_port(SMTP_UI_PORT)
        .with_starttls()
        .build()
    )

    ls_id = f"ls-example-api-{uuid.uuid4().hex[:8]}"
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

    mssql_cs = MSSQL_CONNECTION_STRING_LOCAL
    oracle_cs = ORACLE_CONNECTION_STRING_LOCAL
    pg_cs = (
        f"host=localhost port={POSTGRES_PORT} user={POSTGRES_DB_USER} "
        f"password={POSTGRES_DB_PASS} dbname={POSTGRES_DB_NAME}"
    )
    ls_ep = f"http://127.0.0.1:{LOCALSTACK_HOST_PORT}"

    fastapi_component = (
        ExecutableComponentBuilder(COMPONENT_NAME_EXECUTABLE)
        .with_executable_path(exe)
        .with_env_var("WEB_APP_PORT", str(WEB_APP_PORT))
        .with_env_var("POSTGRES_CONNECTION_STRING", pg_cs)
        .with_env_var("CALIBRATION_URL", f"http://127.0.0.1:{CALIBRATION_HOST_PORT}")
        .with_env_var("MSSQL_CONNECTION_STRING", mssql_cs)
        .with_env_var("ORACLE_CONNECTION_STRING", oracle_cs)
        .with_env_var("TEMPORAL_TARGET", f"127.0.0.1:{TEMPORAL_GRPC_PORT}")
        .with_env_var("SMTP_HOST", "127.0.0.1")
        .with_env_var("SMTP_PORT", str(SMTP_HOST_PORT))
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
            HttpReadinessCheck.create(), f"http://127.0.0.1:{WEB_APP_PORT}/health", 30_000
        )
        .build()
    )

    a_match = (
        MatchBuilder(MATCH_NAME)
        .add_dependency(oauth)
        .add_dependency(postgres)
        .add_dependency(mssql)
        .add_dependency(oracle)
        .add_dependency(calibration)
        .add_dependency(localstack)
        .add_dependency(temporal)
        .add_dependency(smtp)
        .add_component(fastapi_component)
        .register_playbook(
            CalibrationApiHappyPathPlaybook(calibration.identifier),
            exec_on_dependency_start=True,
        )
        .register_playbook(CalibrationApiErrorPathPlaybook(calibration.identifier))
        .register_playbook(CalibrationApiFlakyPlaybook(calibration.identifier))
        .register_playbook(
            EventsPurgePlaybook(localstack.identifier),
            exec_on_dependency_start=True,
        )
        .register_playbook(ResetValidationDbPlaybook(mssql.identifier))
        .register_playbook(SeedValidationReadingPlaybook(mssql_cs))
        .register_playbook(ResetWeatherDbPlaybook(oracle.identifier))
        .build()
    )

    return ClosedArena(
        CLOSED_ARENA_NAME,
        [a_match],
        log_level=ArenaLogLevel.WARN,
        logger=_LOG,
        log_component_ids=(COMPONENT_NAME_EXECUTABLE,),
        log_dependency_ids=(
            oauth.identifier,
            postgres.identifier,
            mssql.identifier,
            oracle.identifier,
            calibration.identifier,
            localstack.identifier,
            temporal.identifier,
            smtp.identifier,
        ),
    )


@pytest.fixture(scope="session")
def base_url() -> str:
    return f"http://127.0.0.1:{WEB_APP_PORT}"


@pytest.fixture(scope="session")
def mssql_connection_string() -> str:
    return MSSQL_CONNECTION_STRING_LOCAL


oauth_signer = oauth_signer_fixture(lambda: _OAUTH)


@pytest.fixture(scope="session")
async def api_client(oauth_signer, base_url) -> ApiClient:
    import requests

    session = requests.Session()
    token = await oauth_signer.sign(Custom(), claims_with_scope(OAUTH_ISSUER, "readings"))
    session.headers.update({"Authorization": f"Bearer {token}"})
    return ApiClient(base_url, session)


@pytest.fixture(scope="session")
def readings_device_id(api_client: ApiClient) -> int:
    return api_client.create_device("Readings Component Test Device")


@pytest.fixture(scope="session")
def _sqs_reading_created_client(arena):
    _ = arena
    boto3 = pytest.importorskip("boto3")
    ls_ep = f"http://127.0.0.1:{LOCALSTACK_HOST_PORT}"
    sqs = boto3.client(
        "sqs",
        region_name=REGION,
        endpoint_url=ls_ep,
        **DUMMY_CREDS,
    )
    queue_url = sqs.get_queue_url(QueueName=QUEUE_NAME)["QueueUrl"]
    return sqs, queue_url


@pytest.fixture(scope="session")
def wait_reading_created_event(_sqs_reading_created_client):
    sqs, queue_url = _sqs_reading_created_client

    def wait(expected_id: int) -> dict:
        return _wait_sqs_reading_created(sqs, queue_url, expected_id)

    return wait
