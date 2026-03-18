import os

import pytest

from arena_pytest import (
    BuildTool,
    ClosedArena,
    EncounterBuilder,
    ExecutableComponentBuilder,
    KafkaDependencyBuilder,
    KafkaFlavor,
    PostgresDependencyBuilder,
)

POSTGRES_PORT = 5555
DB_NAME = "test_database"
DB_USER = "test_user"
DB_PASS = "test_password"
KAFKA_PORT = 9093
EXEC_WEB_APP_PORT = 3000
NETWORK_NAME = "arena-component-test-network"
POSTGRES_CONTAINER_NAME = "arena-component-test-postgres"
KAFKA_CONTAINER_NAME = "arena-component-test-kafka"
KAFKA_TOPIC = "readings"


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


@pytest.fixture(scope="session")
def closed_arena() -> ClosedArena:
    schema_path = _find_schema_path()
    startup_sql = [open(schema_path).read()] if schema_path else []

    postgres = (
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

    kafka = (
        KafkaDependencyBuilder("test kafka")
        .with_flavor(KafkaFlavor.APACHE_NATIVE)
        .with_port(KAFKA_PORT)
        .with_container_name(KAFKA_CONTAINER_NAME)
        .with_topic(KAFKA_TOPIC)
        .build()
    )

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
        .with_readiness_check_url(healthcheck_url)
    )
    if not is_bazel:
        exec_builder = exec_builder.with_source_path("examples").with_build_tool(BuildTool.CARGO)

    exec_component = exec_builder.build()

    encounter = (
        EncounterBuilder("reading lifecycle")
        .with_network(NETWORK_NAME)
        .add_dependency(postgres)
        .add_dependency(kafka)
        .add_component(exec_component)
        .build()
    )

    return ClosedArena("Test Arena", [encounter])


@pytest.fixture(scope="session")
def arena(closed_arena):
    import asyncio
    from arena_pytest import wait_for_http_ready

    loop = asyncio.new_event_loop()
    asyncio.set_event_loop(loop)
    open_arena = loop.run_until_complete(closed_arena.open())
    if open_arena is None or not open_arena.is_valid():
        pytest.skip("arena_open failed (Docker required for dependencies)")
    wait_for_http_ready(f"http://127.0.0.1:{EXEC_WEB_APP_PORT}/readings")
    yield open_arena
    loop.run_until_complete(open_arena.close())
