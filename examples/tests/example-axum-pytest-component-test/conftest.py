from __future__ import annotations

import logging
import os
import sys

_TESTS_DIR = os.path.dirname(os.path.abspath(__file__))
if _TESTS_DIR not in sys.path:
    sys.path.insert(0, _TESTS_DIR)

import tempfile

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

from api_http import ApiClient
from playbook_fixtures import active_http_playbook
from playbooks import (
    CalibrationApiErrorPathPlaybook,
    CalibrationApiHappyPathPlaybook,
    CalibrationApiFlakyPlaybook,
    ResetValidationDbPlaybook,
)

from arena_config import (
    CALIBRATION_CONTAINER_NAME,
    CALIBRATION_HOST_PORT,
    CALIBRATION_VALIDATE_PATH,
    CLOSED_ARENA_NAME,
    COMPONENT_NAME_EXECUTABLE,
    DEP_NAME_CALIBRATION_HTTP,
    DEP_NAME_KAFKA,
    DEP_NAME_MSSQL,
    DEP_NAME_OAUTH,
    DEP_NAME_POSTGRES,
    EXEC_WEB_APP_PORT,
    KAFKA_CONSUMER_GROUP_LABEL,
    KAFKA_CONTAINER_NAME,
    KAFKA_PORT,
    KAFKA_TOPIC,
    MATCH_NAME,
    MSSQL_CONNECTION_STRING_LOCAL,
    MSSQL_CONTAINER_NAME,
    MSSQL_DB_NAME,
    MSSQL_DB_PASS,
    MSSQL_DB_USER,
    MSSQL_PORT,
    OAUTH_ISSUER,
    OAUTH_PORT,
    POSTGRES_CONTAINER_NAME,
    POSTGRES_DB_NAME,
    POSTGRES_DB_PASS,
    POSTGRES_DB_USER,
    POSTGRES_IMAGE,
    POSTGRES_PORT,
)

from arena_pytest import (
    ArenaLogLevel,
    BuildTool,
    ClosedArena,
    ExecutableComponentBuilder,
    HttpDependencyBuilder,
    HttpReadinessCheck,
    KafkaDependencyBuilder,
    KafkaFlavor,
    MatchBuilder,
    MssqlDependencyBuilder,
    OauthDependencyBuilder,
    PostgresDependencyBuilder,
)
_LOG = logging.getLogger(__name__)

_OAUTH_TLS_CERT_PEM: str | None = None
_OAUTH_TLS_KEY_PEM: str | None = None
_OAUTH_TLS_CA_FILE: str | None = None


def _arena_oauth_tls_session() -> tuple[str, str, str]:
    global _OAUTH_TLS_CERT_PEM, _OAUTH_TLS_KEY_PEM, _OAUTH_TLS_CA_FILE
    if _OAUTH_TLS_CERT_PEM is None:
        from arena_pytest import oauth_loopback_tls_pem_pair

        ca_pem, server_key_pem = oauth_loopback_tls_pem_pair()
        fd, path = tempfile.mkstemp(prefix="example-api-oauth-ca-", suffix=".pem")
        with os.fdopen(fd, "w", encoding="utf-8") as f:
            f.write(ca_pem)
        _OAUTH_TLS_CERT_PEM, _OAUTH_TLS_KEY_PEM, _OAUTH_TLS_CA_FILE = ca_pem, server_key_pem, path
    return _OAUTH_TLS_CERT_PEM, _OAUTH_TLS_KEY_PEM, _OAUTH_TLS_CA_FILE


def _find_schema_path(filename: str = "instrument_reading_db_schema.sql") -> str:
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
    examples_root = os.path.dirname(
        os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    )
    p = os.path.join(examples_root, "resources", filename)
    return p if os.path.isfile(p) else ""


def _find_web_app_binary() -> str:
    runfiles_dir = os.environ.get("RUNFILES_DIR")
    if runfiles_dir:
        for base in ("_main", "arena", ""):
            p = os.path.join(runfiles_dir, base, "examples", "example-readings-axum-web-app")
            if os.path.isfile(p):
                return p
    repo_root = os.path.dirname(
        os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
    )
    return os.path.join(repo_root, "target", "release", "example-readings-axum-web-app")


def _build_oauth():
    cert, key, _ = _arena_oauth_tls_session()
    return (
        OauthDependencyBuilder(DEP_NAME_OAUTH)
        .with_port(OAUTH_PORT)
        .with_listen_ip("0.0.0.0")
        .with_server_tls_pem(cert, key)
        .with_metadata_base_url(OAUTH_ISSUER)
        .build()
    )


def _build_postgres(startup_sql: list[str]):
    return (
        PostgresDependencyBuilder(DEP_NAME_POSTGRES)
            .with_image(POSTGRES_IMAGE)
        .with_port(POSTGRES_PORT)
        .with_database_name(POSTGRES_DB_NAME)
        .with_database_username(POSTGRES_DB_USER)
        .with_database_password(POSTGRES_DB_PASS)
        .with_container_name(POSTGRES_CONTAINER_NAME)
        .with_startup_sql_scripts(startup_sql)
        .build()
    )


def _build_kafka():
    return (
        KafkaDependencyBuilder(DEP_NAME_KAFKA)
        .with_flavor(KafkaFlavor.APACHE_NATIVE)
        .with_port(KAFKA_PORT)
        .with_container_name(KAFKA_CONTAINER_NAME)
        .with_topic(KAFKA_TOPIC)
        .build()
    )


def _build_mssql(startup_sql: list[str]):
    return (
        MssqlDependencyBuilder(DEP_NAME_MSSQL)
        .with_port(MSSQL_PORT)
        .with_database_name(MSSQL_DB_NAME)
        .with_database_username(MSSQL_DB_USER)
        .with_database_password(MSSQL_DB_PASS)
        .with_container_name(MSSQL_CONTAINER_NAME)
        .with_startup_sql_scripts(startup_sql)
        .build()
    )


def _build_calibration_http():
    return (
        HttpDependencyBuilder(DEP_NAME_CALIBRATION_HTTP)
        .with_port(CALIBRATION_HOST_PORT)
        .with_container_name(CALIBRATION_CONTAINER_NAME)
        .build()
    )


def _build_exec_component(oauth_ca_pem: str) -> object:
    web_app_binary = _find_web_app_binary()
    healthcheck_url = f"http://127.0.0.1:{EXEC_WEB_APP_PORT}/health"
    is_bazel = bool(os.environ.get("RUNFILES_DIR"))

    exec_builder = (
        ExecutableComponentBuilder(COMPONENT_NAME_EXECUTABLE)
        .with_executable_path(web_app_binary)
        .with_env_var("RUST_LOG", "info")
        .with_env_var("OAUTH_TLS_CA_PEM", oauth_ca_pem)
        .with_runtime_arg("web_app_port", str(EXEC_WEB_APP_PORT))
        .with_runtime_arg(
            "postgres_connection_string",
            f"host=localhost port={POSTGRES_PORT} user={POSTGRES_DB_USER} password={POSTGRES_DB_PASS} dbname={POSTGRES_DB_NAME}",
        )
        .with_runtime_arg("kafka_bootstrap", f"localhost:{KAFKA_PORT}")
        .with_runtime_arg(
            "calibration_url", f"http://127.0.0.1:{CALIBRATION_HOST_PORT}"
        )
        .with_runtime_arg(
            "mssql_connection_string", MSSQL_CONNECTION_STRING_LOCAL
        )
        .with_runtime_arg("oauth_issuer_url", OAUTH_ISSUER)
        .with_readiness_check(HttpReadinessCheck.create(), healthcheck_url, 30_000)
    )
    if not is_bazel:
        exec_builder = exec_builder.with_source_path("examples").with_build_tool(BuildTool.CARGO)

    return exec_builder.build()


@pytest.fixture(scope="session")
def closed_arena() -> ClosedArena:
    oauth_ca_pem, _, _ = _arena_oauth_tls_session()

    oauth = _build_oauth()

    schema_path = _find_schema_path()
    startup_sql = [open(schema_path).read()] if schema_path else []

    mssql_schema_path = _find_schema_path("validation_db_schema.sql")
    mssql_startup_sql = [open(mssql_schema_path).read()] if mssql_schema_path else []

    calibration_http = _build_calibration_http()
    postgres = _build_postgres(startup_sql)
    kafka = _build_kafka()
    mssql = _build_mssql(mssql_startup_sql)
    calibration_happy_path = CalibrationApiHappyPathPlaybook(calibration_http.identifier)
    calibration_error_path = CalibrationApiErrorPathPlaybook(calibration_http.identifier)
    calibration_flaky_path = CalibrationApiFlakyPlaybook(calibration_http.identifier)
    reset_validation_db = ResetValidationDbPlaybook(mssql.identifier)

    a_match = MatchBuilder(MATCH_NAME)
    a_match = a_match.add_dependency(oauth)
    a_match = a_match.add_dependency(postgres)
    a_match = a_match.add_dependency(kafka)
    a_match = a_match.add_dependency(mssql)
    a_match = a_match.add_dependency(calibration_http)
    a_match = a_match.register_playbook(
        calibration_happy_path,
        exec_on_dependency_start=True,
    )
    a_match = a_match.register_playbook(calibration_error_path)
    a_match = a_match.register_playbook(calibration_flaky_path)
    a_match = a_match.register_playbook(reset_validation_db)
    a_match = a_match.add_component(_build_exec_component(oauth_ca_pem))

    return ClosedArena(
        CLOSED_ARENA_NAME,
        [a_match.build()],
        log_level=ArenaLogLevel.DEBUG,
        logger=_LOG,
        log_component_ids=(COMPONENT_NAME_EXECUTABLE,),
        log_dependency_ids=(
            oauth.identifier,
            postgres.identifier,
            kafka.identifier,
            mssql.identifier,
            calibration_http.identifier,
            calibration_happy_path.identifier,
            calibration_error_path.identifier,
            calibration_flaky_path.identifier,
            reset_validation_db.identifier,
        ),
    )


def _fetch_oauth_access_token() -> str:
    import requests

    _, _, ca = _arena_oauth_tls_session()
    s = requests.Session()
    s.verify = ca
    issuer = OAUTH_ISSUER
    disc = s.get(f"{issuer}/.well-known/oauth-authorization-server", timeout=30)
    disc.raise_for_status()
    token_url = disc.json()["token_endpoint"]
    tok = s.post(
        token_url,
        data={
            "grant_type": "client_credentials",
            "client_id": "arena-examples",
        },
        timeout=30,
    )
    tok.raise_for_status()
    return str(tok.json()["access_token"])


@pytest.fixture(scope="session")
def api_client(arena, base_url) -> ApiClient:
    import requests

    _ = arena
    session = requests.Session()
    session.headers.update({"Authorization": f"Bearer {_fetch_oauth_access_token()}"})
    return ApiClient(base_url, session)


def _new_kafka_consumer(bootstrap: str, topic: str, group_prefix: str):
    import os

    from kafka import KafkaConsumer

    return KafkaConsumer(
        topic,
        bootstrap_servers=bootstrap,
        group_id=f"{KAFKA_CONSUMER_GROUP_LABEL}-{group_prefix}-{os.getpid()}",
        auto_offset_reset="earliest",
    )


def _consume_reading_created_event(
    bootstrap: str,
    topic: str,
    group_prefix: str,
    expected_id: int,
    timeout: float = 5.0,
) -> dict:
    import json
    import time

    consumer = _new_kafka_consumer(bootstrap, topic, group_prefix)
    try:
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


@pytest.fixture(scope="session")
def base_url() -> str:
    return f"http://127.0.0.1:{EXEC_WEB_APP_PORT}"


@pytest.fixture(scope="session")
def wait_reading_created_event():
    bootstrap = f"localhost:{KAFKA_PORT}"

    def wait(expected_id: int) -> dict:
        return _consume_reading_created_event(
            bootstrap,
            KAFKA_TOPIC,
            "exec",
            expected_id,
        )

    return wait
