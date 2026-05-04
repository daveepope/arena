import os

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
)
from arena_pytest.oauth import oauth_issuer_host_is_non_loopback

POSTGRES_PORT = 5556
POSTGRES_DB_NAME = "readings_db"
POSTGRES_DB_USER = "readings_user"
POSTGRES_DB_PASS = "readings_password"
KAFKA_PORT = 9094
MSSQL_PORT = 1436
MSSQL_DB_NAME = "validationDb"
MSSQL_DB_USER = "sa"
MSSQL_DB_PASS = "yourStrong(!)Password"
EXEC_WEB_APP_PORT = 3001
DOCKER_WEB_HOST_PORT = 3002
NETWORK_NAME = "arena-pytest-network"
POSTGRES_CONTAINER_NAME = "arena-pytest-postgres"
KAFKA_CONTAINER_NAME = "arena-pytest-kafka"
MSSQL_CONTAINER_NAME = "arena-pytest-mssql"
CALIBRATION_CONTAINER_NAME = "arena-pytest-calibration"
CALIBRATION_HOST_PORT = 3003
CALIBRATION_VALIDATE_PATH = "/api/v1/validate"
KAFKA_TOPIC = "readings"
DOCKER_IMAGE_TAG = "arena-pytest-docker-webapp"
MSSQL_CONNECTION_STRING_LOCAL = (
    f"Server=tcp:localhost,{MSSQL_PORT};Database={MSSQL_DB_NAME};"
    f"User Id={MSSQL_DB_USER};Password={MSSQL_DB_PASS};TrustServerCertificate=True;"
)
MSSQL_CONNECTION_STRING_DOCKER = (
    f"Server=tcp:{MSSQL_CONTAINER_NAME},1433;Database={MSSQL_DB_NAME};"
    f"User Id={MSSQL_DB_USER};Password={MSSQL_DB_PASS};TrustServerCertificate=True;"
)

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
    ctx = tempfile.mkdtemp(prefix="arena-pytest-image-ctx-")
    dst = os.path.join(ctx, "example-readings-axum-web-app")
    shutil.copy2(binary, dst)
    os.chmod(dst, 0o755)
    return ctx, _RUNTIME_CONTAINERFILE


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


def _read_oauth_tls_pem_pair() -> tuple[str, str]:
    anchor = _find_schema_path("instrument_reading_db_schema.sql")
    if not anchor:
        return "", ""
    d = os.path.dirname(anchor)
    cert_p = os.path.join(d, "oauth_tls_cert.pem")
    key_p = os.path.join(d, "oauth_tls_key.pem")
    if not (os.path.isfile(cert_p) and os.path.isfile(key_p)):
        return "", ""
    with open(cert_p, encoding="utf-8") as f:
        cert = f.read()
    with open(key_p, encoding="utf-8") as f:
        key = f.read()
    return cert, key


def _oauth_tls_ca_path() -> str:
    anchor = _find_schema_path("instrument_reading_db_schema.sql")
    if not anchor:
        return ""
    p = os.path.join(os.path.dirname(anchor), "oauth_tls_cert.pem")
    return p if os.path.isfile(p) else ""


def _build_oauth():
    cert, key = _read_oauth_tls_pem_pair()
    if not cert or not key:
        raise FileNotFoundError(
            "examples/resources/oauth_tls_cert.pem and oauth_tls_key.pem are required "
            "for arena-pytest (readings axum sample validates JWTs against JWKS)."
        )
    return (
        OauthDependencyBuilder("pytest oauth")
        .with_port(DEFAULT_OAUTH_PORT)
        .with_listen_ip("0.0.0.0")
        .with_server_tls_pem(cert, key)
        .build()
    )


def _build_postgres(startup_sql: list[str]):
    return (
        PostgresDependencyBuilder("pytest readings")
        .with_image("14.20-trixie")
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
        KafkaDependencyBuilder("pytest readings")
        .with_flavor(KafkaFlavor.APACHE_NATIVE)
        .with_port(KAFKA_PORT)
        .with_container_name(KAFKA_CONTAINER_NAME)
        .with_topic(KAFKA_TOPIC)
        .build()
    )


def _build_mssql(startup_sql: list[str]):
    return (
        MssqlDependencyBuilder("pytest validation")
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
        HttpDependencyBuilder("pytest calibration")
        .with_port(CALIBRATION_HOST_PORT)
        .with_container_name(CALIBRATION_CONTAINER_NAME)
        .build()
    )


def _build_calibration_playbook(dependency_identifier: str):
    return (
        ManagedHttpPlaybookBuilder(
            "calibration-default", dependency_identifier
        )
        .with_mapping("POST", CALIBRATION_VALIDATE_PATH, 200, {"valid": True})
        .build()
    )


@pytest.fixture(scope="session")
def calibration_outage_playbook(calibration_identifier):
    return (
        ManagedHttpPlaybookBuilder(
            "calibration-outage", calibration_identifier
        )
        .with_mapping("POST", CALIBRATION_VALIDATE_PATH, status=500)
        .build()
    )


@pytest.fixture(scope="session")
def validation_db_playbook(mssql_identifier):
    return ManagedMssqlPlaybookBuilder(
        "validation-db-scoped", mssql_identifier
    ).build()


def _build_exec_component(oauth_ca_pem: str) -> object:
    web_app_binary = _find_web_app_binary()
    healthcheck_url = f"http://127.0.0.1:{EXEC_WEB_APP_PORT}/health"
    is_bazel = bool(os.environ.get("RUNFILES_DIR"))

    exec_builder = (
        ExecutableComponentBuilder("pytest web app")
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
        .with_readiness_check(HttpReadinessCheck(), healthcheck_url)
    )
    if not is_bazel:
        exec_builder = exec_builder.with_source_path("examples").with_build_tool(BuildTool.CARGO)

    return exec_builder.build()


def _build_containerized_component(
    repo_root: str, containerfile: str, oauth_ca_pem: str
) -> object:
    return (
        ContainerizedComponentBuilder("pytest web app container", containerfile)
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
            HttpReadinessCheck(),
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

    oauth_ca_pem, _oauth_key_pem = _read_oauth_tls_pem_pair()
    if not oauth_ca_pem:
        pytest.fail(
            "Missing examples/resources/oauth_tls_cert.pem (and oauth_tls_key.pem). "
            "They are required for the readings axum sample OAuth stack when running pytest with Arena fixtures."
        )
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

    a_match = MatchBuilder("reading lifecycle")
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

    return ClosedArena("Test Arena", [a_match.build()])


@pytest.fixture(scope="session")
def oauth_access_token(arena) -> str:
    import requests

    ca = _oauth_tls_ca_path()
    if not ca:
        pytest.fail("oauth TLS CA file missing (examples/resources/oauth_tls_cert.pem)")
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
