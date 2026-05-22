import os
import sys
import logging

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

from arena_pytest import (
    ArenaLogLevel,
    BuildTool,
    ClosedArena,
    ContainerizedComponentBuilder,
    DEFAULT_OAUTH_PORT,
    ExecutableComponentBuilder,
    HttpDependencyBuilder,
    HttpReadinessCheck,
    KafkaDependencyBuilder,
    KafkaFlavor,
    KAFKA_INTERNAL_DOCKER_PORT,
    ManagedHttpPlaybookBuilder,
    ManagedMssqlPlaybookBuilder,
    MatchBuilder,
    MssqlDependencyBuilder,
    OAUTH_ISSUER,
    OauthDependencyBuilder,
    PostgresDependencyBuilder,
    active_playbooks,
)
from arena_pytest.oauth import oauth_issuer_host_is_non_loopback

from readings_arena_config import (
    CALIBRATION_CONTAINER_NAME,
    CALIBRATION_HOST_PORT,
    CALIBRATION_VALIDATE_PATH,
    CLOSED_ARENA_NAME,
    COMPONENT_NAME_CONTAINERIZED,
    COMPONENT_NAME_EXECUTABLE,
    DEP_NAME_CALIBRATION_HTTP,
    DEP_NAME_KAFKA,
    DEP_NAME_MSSQL,
    DEP_NAME_OAUTH,
    DEP_NAME_POSTGRES,
    DOCKER_IMAGE_TAG,
    DOCKER_WEB_HOST_PORT,
    EXEC_WEB_APP_PORT,
    KAFKA_CONTAINER_NAME,
    KAFKA_PORT,
    KAFKA_TOPIC,
    MATCH_NAME,
    MSSQL_CONNECTION_STRING_DOCKER,
    MSSQL_CONNECTION_STRING_LOCAL,
    MSSQL_CONTAINER_NAME,
    MSSQL_DB_NAME,
    MSSQL_DB_PASS,
    MSSQL_DB_USER,
    MSSQL_PORT,
    NETWORK_NAME,
    PLAYBOOK_CALIBRATION_DEFAULT,
    PLAYBOOK_CALIBRATION_OUTAGE_FIXTURE_SCOPE,
    PLAYBOOK_CALIBRATION_OUTAGE_MANAGED,
    PLAYBOOK_VALIDATION_DB_SCOPED,
    POSTGRES_CONTAINER_NAME,
    POSTGRES_DB_NAME,
    POSTGRES_DB_PASS,
    POSTGRES_DB_USER,
    POSTGRES_IMAGE,
    POSTGRES_PORT,
    TEMP_DIRECTORY_PREFIX,
)

_DISPATCHER_IMPLEMENTATION_DEPENDENCY_LOG_IDS = (
    DEP_NAME_OAUTH,
    DEP_NAME_POSTGRES,
    DEP_NAME_KAFKA,
    DEP_NAME_MSSQL,
    DEP_NAME_CALIBRATION_HTTP,
    PLAYBOOK_CALIBRATION_DEFAULT,
    PLAYBOOK_CALIBRATION_OUTAGE_MANAGED,
    PLAYBOOK_CALIBRATION_OUTAGE_FIXTURE_SCOPE,
    PLAYBOOK_VALIDATION_DB_SCOPED,
)

_DISPATCHER_IMPLEMENTATION_COMPONENT_LOG_IDS = (
    COMPONENT_NAME_EXECUTABLE,
    COMPONENT_NAME_CONTAINERIZED,
)

_LOG = logging.getLogger(__name__)

_DOCKER_WEB_ENABLED = False


_RUNTIME_CONTAINERFILE = """\
FROM debian:trixie-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY example-readings-axum-web-app /usr/local/bin/example-readings-axum-web-app
RUN chmod +x /usr/local/bin/example-readings-axum-web-app
EXPOSE 3000
ENTRYPOINT ["/usr/local/bin/example-readings-axum-web-app"]
"""


def _prepare_container_image_context() -> tuple[str, str]:
    """Stage a minimal build context: the prebuilt readings axum binary + runtime containerfile.

    Returns (build_context_dir, containerfile_text) or ("", "") if the binary isn't found.
    """
    import shutil
    import tempfile

    binary = _find_web_app_binary()
    if not binary or not os.path.isfile(binary):
        return "", ""
    ctx = tempfile.mkdtemp(prefix=TEMP_DIRECTORY_PREFIX)
    dst = os.path.join(ctx, "example-readings-axum-web-app")
    shutil.copy2(binary, dst)
    os.chmod(dst, 0o755)
    return ctx, _RUNTIME_CONTAINERFILE


_OAUTH_TLS_CERT_PEM: str | None = None
_OAUTH_TLS_KEY_PEM: str | None = None
_OAUTH_TLS_CA_FILE: str | None = None


def _arena_oauth_tls_session() -> tuple[str, str, str]:
    global _OAUTH_TLS_CERT_PEM, _OAUTH_TLS_KEY_PEM, _OAUTH_TLS_CA_FILE
    if _OAUTH_TLS_CERT_PEM is None:
        from arena_pytest import oauth_loopback_tls_pem_pair

        ca_pem, server_key_pem = oauth_loopback_tls_pem_pair()
        fd, path = tempfile.mkstemp(prefix="arena-pytest-oauth-ca-", suffix=".pem")
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
    root = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
    p = os.path.join(root, "examples", "resources", filename)
    return p if os.path.isfile(p) else ""


def _find_web_app_binary() -> str:
    runfiles_dir = os.environ.get("RUNFILES_DIR")
    if runfiles_dir:
        for base in ("_main", "arena", ""):
            p = os.path.join(runfiles_dir, base, "examples", "example-readings-axum-web-app")
            if os.path.isfile(p):
                return p
    root = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
    return os.path.join(root, "target", "release", "example-readings-axum-web-app")


def _build_oauth():
    cert, key, _ = _arena_oauth_tls_session()
    return (
        OauthDependencyBuilder(DEP_NAME_OAUTH)
        .with_port(DEFAULT_OAUTH_PORT)
        .with_listen_ip("0.0.0.0")
        .with_server_tls_pem(cert, key)
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


def _build_calibration_playbook(dependency_identifier: str):
    return (
        ManagedHttpPlaybookBuilder(
            PLAYBOOK_CALIBRATION_DEFAULT, dependency_identifier
        )
        .with_mapping("POST", CALIBRATION_VALIDATE_PATH, 200, {"valid": True})
        .build()
    )


@pytest.fixture(scope="session")
def calibration_outage_playbook(calibration_identifier):
    return (
        ManagedHttpPlaybookBuilder(
            PLAYBOOK_CALIBRATION_OUTAGE_MANAGED, calibration_identifier
        )
        .with_mapping("POST", CALIBRATION_VALIDATE_PATH, status=500)
        .build()
    )


@pytest.fixture(scope="session")
def validation_db_playbook(mssql_identifier):
    return ManagedMssqlPlaybookBuilder(
        PLAYBOOK_VALIDATION_DB_SCOPED, mssql_identifier
    ).build()


@pytest.fixture(scope="session")
def calibration_outage_fixture_scope_playbook(calibration_identifier):
    return (
        ManagedHttpPlaybookBuilder(
            PLAYBOOK_CALIBRATION_OUTAGE_FIXTURE_SCOPE, calibration_identifier
        )
        .with_mapping("POST", CALIBRATION_VALIDATE_PATH, status=500)
        .build()
    )


@pytest.fixture
def outage_and_db_reset(
    arena, calibration_outage_fixture_scope_playbook, validation_db_playbook
):
    with active_playbooks(
        arena, calibration_outage_fixture_scope_playbook, validation_db_playbook
    ):
        yield


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
        .with_readiness_check(HttpReadinessCheck.create(), healthcheck_url)
    )
    if not is_bazel:
        exec_builder = exec_builder.with_source_path("examples").with_build_tool(BuildTool.CARGO)

    return exec_builder.build()


def _build_containerized_component(
    repo_root: str, containerfile: str, oauth_ca_pem: str
) -> object:
    return (
        ContainerizedComponentBuilder(COMPONENT_NAME_CONTAINERIZED, containerfile)
        .with_build_context(repo_root)
        .with_image_tag(DOCKER_IMAGE_TAG)
        .with_network(NETWORK_NAME)
        .with_port_mapping(DOCKER_WEB_HOST_PORT, 3000)
        .with_host_mapping("host.docker.internal:host-gateway")
        .with_env_var("RUST_LOG", "info")
        .with_env_var("OAUTH_TLS_CA_PEM", oauth_ca_pem)
        .with_runtime_arg("web_app_port", "3000")
        .with_runtime_arg(
            "postgres_connection_string",
            f"host={POSTGRES_CONTAINER_NAME} port=5432 user={POSTGRES_DB_USER} password={POSTGRES_DB_PASS} dbname={POSTGRES_DB_NAME}",
        )
        .with_runtime_arg(
            "kafka_bootstrap",
            f"{KAFKA_CONTAINER_NAME}:{KAFKA_INTERNAL_DOCKER_PORT}",
        )
        .with_runtime_arg(
            "calibration_url", f"http://{CALIBRATION_CONTAINER_NAME}:8080"
        )
        .with_runtime_arg(
            "mssql_connection_string", MSSQL_CONNECTION_STRING_DOCKER
        )
        .with_runtime_arg("oauth_issuer_url", OAUTH_ISSUER)
        .with_readiness_check(
            HttpReadinessCheck.create(),
            f"http://127.0.0.1:{DOCKER_WEB_HOST_PORT}/health",
        )
        .build()
    )


_CALIBRATION_IDENTIFIER: str | None = None
_MSSQL_IDENTIFIER: str | None = None


@pytest.fixture(scope="session")
def docker_web_enabled(closed_arena) -> bool:
    _ = closed_arena
    return _DOCKER_WEB_ENABLED


@pytest.fixture(scope="session")
def calibration_identifier() -> str:
    assert _CALIBRATION_IDENTIFIER is not None, (
        "calibration_identifier requested before closed_arena fixture built it"
    )
    return _CALIBRATION_IDENTIFIER


@pytest.fixture(scope="session")
def mssql_identifier() -> str:
    assert _MSSQL_IDENTIFIER is not None, (
        "mssql_identifier requested before closed_arena fixture built it"
    )
    return _MSSQL_IDENTIFIER


@pytest.fixture(scope="session")
def closed_arena() -> ClosedArena:
    global _CALIBRATION_IDENTIFIER, _MSSQL_IDENTIFIER, _DOCKER_WEB_ENABLED

    oauth_ca_pem, _, _ = _arena_oauth_tls_session()

    oauth = _build_oauth()

    schema_path = _find_schema_path()
    startup_sql = [open(schema_path).read()] if schema_path else []

    mssql_schema_path = _find_schema_path("validation_db_schema.sql")
    mssql_startup_sql = [open(mssql_schema_path).read()] if mssql_schema_path else []

    components = [_build_exec_component(oauth_ca_pem)]

    _DOCKER_WEB_ENABLED = False
    ctx, containerfile = _prepare_container_image_context()
    if ctx and containerfile and oauth_issuer_host_is_non_loopback:
        components.append(_build_containerized_component(ctx, containerfile, oauth_ca_pem))
        _DOCKER_WEB_ENABLED = True

    calibration_http = _build_calibration_http()
    _CALIBRATION_IDENTIFIER = calibration_http.identifier

    mssql = _build_mssql(mssql_startup_sql)
    _MSSQL_IDENTIFIER = mssql.identifier

    a_match = MatchBuilder(MATCH_NAME)
    a_match = a_match.with_network(NETWORK_NAME)
    a_match = a_match.add_dependency(oauth)
    a_match = a_match.add_dependency(_build_postgres(startup_sql))
    a_match = a_match.add_dependency(_build_kafka())
    a_match = a_match.add_dependency(mssql)
    a_match = a_match.add_dependency(calibration_http)
    a_match = a_match.register_playbook(
        _build_calibration_playbook(calibration_http.identifier),
        exec_on_dependency_start=True,
    )
    for c in components:
        a_match = a_match.add_component(c)

    return ClosedArena(
        CLOSED_ARENA_NAME,
        [a_match.build()],
        log_level=ArenaLogLevel.DEBUG,
        logger=_LOG,
        log_component_ids=_DISPATCHER_IMPLEMENTATION_COMPONENT_LOG_IDS,
        log_dependency_ids=_DISPATCHER_IMPLEMENTATION_DEPENDENCY_LOG_IDS,
    )


@pytest.fixture(scope="session")
def oauth_access_token(arena) -> str:
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
def arena_docker(arena):
    """Same session arena as ``arena`` (exec + container when Dockerfile is available)."""
    yield arena
