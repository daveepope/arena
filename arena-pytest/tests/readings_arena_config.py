import json
import os

from readings_ephemeral_test_runtime import RUNTIME


def _find_config_json_path() -> str:
    try:
        from bazel_tools.tools.python.runfiles import runfiles

        r = runfiles.Create()
        for rel in (
            "arena/examples/resources/readings_arena_config.json",
            "_main/examples/resources/readings_arena_config.json",
        ):
            p = r.Rlocation(rel)
            if p and os.path.isfile(p):
                return p
    except ImportError:
        pass
    runfiles_dir = os.environ.get("RUNFILES_DIR")
    if runfiles_dir:
        for base in ("_main", "arena", ""):
            parts = [runfiles_dir]
            if base:
                parts.append(base)
            parts.extend(["examples", "resources", "readings_arena_config.json"])
            p = os.path.join(*parts)
            if os.path.isfile(p):
                return p
    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    p = os.path.join(root, "examples", "resources", "readings_arena_config.json")
    return p if os.path.isfile(p) else ""


def _load_raw() -> dict:
    path = _find_config_json_path()
    if not path:
        raise RuntimeError(
            "readings_arena_config.json not found (run via Bazel or from repo root)"
        )
    with open(path, encoding="utf-8") as f:
        return json.load(f)


_DATA = _load_raw()
_DATABASE = _DATA["database"]
_DEP_NAMES = _DATA["dependency_names"]
_COMP_NAMES = _DATA["component_names"]
_CTR_NAMES = _DATA["container_names"]
_PB_NAMES = _DATA["playbook_names"]

NETWORK_NAME = RUNTIME.network_name(_DATA["network_name"])
DOCKER_IMAGE_TAG = _DATA["docker_image_tag"]
CLOSED_ARENA_NAME = _DATA["closed_arena_name"]
MATCH_NAME = _DATA["match_name"]
TEMP_DIRECTORY_PREFIX = _DATA["temp_directory_prefix"]
KAFKA_CONSUMER_GROUP_LABEL = _DATA["kafka_consumer_group_label"]
POSTGRES_IMAGE = _DATA["postgres_image"]

EXEC_WEB_APP_PORT = RUNTIME.exec_web_app_port
DOCKER_WEB_HOST_PORT = RUNTIME.docker_web_host_port
KAFKA_PORT = RUNTIME.kafka_port
CALIBRATION_HOST_PORT = RUNTIME.calibration_host_port
POSTGRES_PORT = RUNTIME.postgres_port
MSSQL_PORT = RUNTIME.mssql_port
OAUTH_PORT = RUNTIME.oauth_port
OAUTH_ISSUER = RUNTIME.oauth_issuer
LOCALSTACK_HOST_PORT = RUNTIME.localstack_host_port

POSTGRES_DB_NAME = _DATABASE["postgres_name"]
POSTGRES_DB_USER = _DATABASE["postgres_user"]
POSTGRES_DB_PASS = _DATABASE["postgres_password"]
MSSQL_DB_NAME = _DATABASE["mssql_name"]
MSSQL_DB_USER = _DATABASE["mssql_user"]
MSSQL_DB_PASS = _DATABASE["mssql_password"]

POSTGRES_CONTAINER_NAME = RUNTIME.container_name(_CTR_NAMES["postgres"])
KAFKA_CONTAINER_NAME = RUNTIME.container_name(_CTR_NAMES["kafka"])
MSSQL_CONTAINER_NAME = RUNTIME.container_name(_CTR_NAMES["mssql"])
CALIBRATION_CONTAINER_NAME = RUNTIME.container_name(_CTR_NAMES["calibration"])

KAFKA_TOPIC = _DATA["kafka_topic"]
CALIBRATION_VALIDATE_PATH = _DATA["calibration_validate_path"]

DEP_NAME_OAUTH = _DEP_NAMES["oauth"]
DEP_NAME_POSTGRES = _DEP_NAMES["postgres"]
DEP_NAME_KAFKA = _DEP_NAMES["kafka"]
DEP_NAME_MSSQL = _DEP_NAMES["mssql"]
DEP_NAME_CALIBRATION_HTTP = _DEP_NAMES["calibration_http"]

COMPONENT_NAME_EXECUTABLE = _COMP_NAMES["executable"]
COMPONENT_NAME_CONTAINERIZED = _COMP_NAMES["containerized"]

PLAYBOOK_CALIBRATION_DEFAULT = _PB_NAMES["calibration_default"]
PLAYBOOK_CALIBRATION_OUTAGE_MANAGED = _PB_NAMES["calibration_outage_managed"]
PLAYBOOK_CALIBRATION_OUTAGE_FIXTURE_SCOPE = _PB_NAMES["calibration_outage_fixture_scope"]
PLAYBOOK_VALIDATION_DB_SCOPED = _PB_NAMES["validation_db_scoped"]
PLAYBOOK_LOCALSTACK_SESSION = _PB_NAMES["localstack_session"]

MSSQL_CONNECTION_STRING_LOCAL = (
    f"Server=tcp:localhost,{MSSQL_PORT};Database={MSSQL_DB_NAME};"
    f"User Id={MSSQL_DB_USER};Password={MSSQL_DB_PASS};TrustServerCertificate=True;encrypt=DANGER_PLAINTEXT;"
)
MSSQL_CONNECTION_STRING_DOCKER = (
    f"Server=tcp:{MSSQL_CONTAINER_NAME},1433;Database={MSSQL_DB_NAME};"
    f"User Id={MSSQL_DB_USER};Password={MSSQL_DB_PASS};TrustServerCertificate=True;encrypt=DANGER_PLAINTEXT;"
)
