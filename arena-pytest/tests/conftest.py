import os

import pytest
import pytest_asyncio

from arena_pytest import (
    BuildTool,
    ClosedArena,
    ContainerComponentBuilder,
    EncounterBuilder,
    ExecutableComponentBuilder,
    HttpReadinessCheck,
    KafkaDependencyBuilder,
    KafkaFlavor,
    KAFKA_INTERNAL_DOCKER_PORT,
    PostgresDependencyBuilder,
)

POSTGRES_PORT = 5556
DB_NAME = "test_database"
DB_USER = "test_user"
DB_PASS = "test_password"
KAFKA_PORT = 9094
EXEC_WEB_APP_PORT = 3001
DOCKER_WEB_HOST_PORT = 3002
NETWORK_NAME = "arena-pytest-network"
POSTGRES_CONTAINER_NAME = "arena-pytest-postgres"
KAFKA_CONTAINER_NAME = "arena-pytest-kafka"
KAFKA_TOPIC = "readings"
DOCKER_IMAGE_TAG = "arena-pytest-docker-webapp"


def _find_repo_root() -> str:
    try:
        from bazel_tools.tools.python.runfiles import runfiles

        r = runfiles.Create()
        for rel in ("_main", "arena"):
            p = r.Rlocation(rel)
            if p and os.path.isdir(p) and os.path.isfile(os.path.join(p, "Cargo.toml")):
                return p
    except ImportError:
        pass
    runfiles_dir = os.environ.get("RUNFILES_DIR")
    if runfiles_dir:
        for base in ("_main", "arena", ""):
            root = os.path.join(runfiles_dir, base) if base else runfiles_dir
            if os.path.isfile(os.path.join(root, "Cargo.toml")):
                return root
    root = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
    return root if os.path.isfile(os.path.join(root, "Cargo.toml")) else ""


def _find_dockerfile_path() -> str:
    root = _find_repo_root()
    if not root:
        return ""
    p = os.path.join(root, "examples", "src", "example-web-app", "Dockerfile")
    return p if os.path.isfile(p) else ""


def _find_schema_path() -> str:
    try:
        from bazel_tools.tools.python.runfiles import runfiles

        r = runfiles.Create()
        for rel in (
            "arena/examples/resources/instrument_reading_db_schema.sql",
            "_main/examples/resources/instrument_reading_db_schema.sql",
        ):
            p = r.Rlocation(rel)
            if p and os.path.isfile(p):
                return p
    except ImportError:
        pass
    runfiles_dir = os.environ.get("RUNFILES_DIR")
    if runfiles_dir:
        for base in ("_main", "arena", ""):
            p = os.path.join(runfiles_dir, base, "examples", "resources", "instrument_reading_db_schema.sql")
            if os.path.isfile(p):
                return p
    root = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
    p = os.path.join(root, "examples", "resources", "instrument_reading_db_schema.sql")
    return p if os.path.isfile(p) else ""


def _find_web_app_binary() -> str:
    runfiles_dir = os.environ.get("RUNFILES_DIR")
    if runfiles_dir:
        for base in ("_main", "arena", ""):
            p = os.path.join(runfiles_dir, base, "examples", "web-app")
            if os.path.isfile(p):
                return p
    root = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
    return os.path.join(root, "target", "release", "web-app")


def _build_postgres(startup_sql: list[str]):
    return (
        PostgresDependencyBuilder("test database")
        .with_image("14.20-trixie")
        .with_port(POSTGRES_PORT)
        .with_database_name(DB_NAME)
        .with_database_username(DB_USER)
        .with_database_password(DB_PASS)
        .with_container_name(POSTGRES_CONTAINER_NAME)
        .with_startup_sql_scripts(startup_sql)
        .build()
    )


def _build_kafka():
    return (
        KafkaDependencyBuilder("test kafka")
        .with_flavor(KafkaFlavor.APACHE_NATIVE)
        .with_port(KAFKA_PORT)
        .with_container_name(KAFKA_CONTAINER_NAME)
        .with_topic(KAFKA_TOPIC)
        .build()
    )


def _build_exec_component() -> object:
    web_app_binary = _find_web_app_binary()
    healthcheck_url = f"http://127.0.0.1:{EXEC_WEB_APP_PORT}/readings"
    is_bazel = bool(os.environ.get("RUNFILES_DIR"))

    exec_builder = (
        ExecutableComponentBuilder("test web app (exec)")
        .with_executable_path(web_app_binary)
        .with_env_var("RUST_LOG", "info")
        .with_runtime_arg("web_app_port", str(EXEC_WEB_APP_PORT))
        .with_runtime_arg(
            "postgres_connection_string",
            f"host=localhost port={POSTGRES_PORT} user={DB_USER} password={DB_PASS} dbname={DB_NAME}",
        )
        .with_runtime_arg("kafka_bootstrap", f"localhost:{KAFKA_PORT}")
        .with_readiness_check(HttpReadinessCheck(), healthcheck_url)
    )
    if not is_bazel:
        exec_builder = exec_builder.with_source_path("examples").with_build_tool(BuildTool.CARGO)

    return exec_builder.build()


def _build_container_component(repo_root: str, dockerfile: str) -> object:
    return (
        ContainerComponentBuilder("test web app (docker)", dockerfile)
        .with_build_context(repo_root)
        .with_image_tag(DOCKER_IMAGE_TAG)
        .with_network(NETWORK_NAME)
        .with_port_mapping(DOCKER_WEB_HOST_PORT, 3000)
        .with_env_var("RUST_LOG", "info")
        .with_runtime_arg("web_app_port", "3000")
        .with_runtime_arg(
            "postgres_connection_string",
            f"host={POSTGRES_CONTAINER_NAME} port=5432 user={DB_USER} password={DB_PASS} dbname={DB_NAME}",
        )
        .with_runtime_arg(
            "kafka_bootstrap",
            f"{KAFKA_CONTAINER_NAME}:{KAFKA_INTERNAL_DOCKER_PORT}",
        )
        .with_readiness_check(
            HttpReadinessCheck(),
            f"http://127.0.0.1:{DOCKER_WEB_HOST_PORT}/readings",
        )
        .build()
    )


@pytest.fixture(scope="session")
def closed_arena() -> ClosedArena:
    """One arena: shared Postgres + Kafka + network, exec binary + optional Docker web app."""
    schema_path = _find_schema_path()
    startup_sql = [open(schema_path).read()] if schema_path else []

    components = [_build_exec_component()]

    repo_root = _find_repo_root()
    dockerfile_path = _find_dockerfile_path()
    if repo_root and dockerfile_path:
        dockerfile = open(dockerfile_path).read()
        components.append(_build_container_component(repo_root, dockerfile))

    encounter = EncounterBuilder("reading lifecycle")
    encounter = encounter.with_network(NETWORK_NAME)
    encounter = encounter.add_dependency(_build_postgres(startup_sql))
    encounter = encounter.add_dependency(_build_kafka())
    for c in components:
        encounter = encounter.add_component(c)

    return ClosedArena("Test Arena", [encounter.build()])


@pytest_asyncio.fixture(scope="session")
async def arena(closed_arena):
    open_arena = await closed_arena.open()
    if open_arena is None:
        pytest.skip("arena_open failed (Docker required for dependencies)")
    try:
        yield open_arena
    finally:
        await open_arena.close()


@pytest.fixture(scope="session")
def arena_docker(arena):
    """Same session arena as ``arena`` (exec + container when Dockerfile is available)."""
    yield arena
