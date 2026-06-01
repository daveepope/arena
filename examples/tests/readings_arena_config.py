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
    p = os.path.join(root, "resources", "readings_arena_config.json")
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
_PB_NAMES = _DATA["playbook_names"]

CALIBRATION_VALIDATE_PATH = _DATA["calibration_validate_path"]

PLAYBOOK_CALIBRATION_DEFAULT = _PB_NAMES["calibration_default"]
PLAYBOOK_CALIBRATION_OUTAGE_MANAGED = _PB_NAMES["calibration_outage_managed"]
PLAYBOOK_VALIDATION_DB_SCOPED = _PB_NAMES["validation_db_scoped"]
PLAYBOOK_LOCALSTACK_SESSION = _PB_NAMES["localstack_session"]
