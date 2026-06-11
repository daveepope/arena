#!/usr/bin/env python3

from __future__ import annotations

import os
import sys
from pathlib import Path

from arena_version import prepare_release_version, resolve_semver_level_from_env


def _repo_root() -> Path:
    ws = os.environ.get("BUILD_WORKSPACE_DIRECTORY")
    if ws:
        return Path(ws)
    return Path(__file__).resolve().parent.parent


def main() -> int:
    root = _repo_root()
    level = resolve_semver_level_from_env()
    master_ref = os.environ.get("ARENA_VERSION_BASE_REF", "origin/master")
    target, changed = prepare_release_version(root, master_ref, level)
    if changed:
        print(f"release VERSION {target} ({level}; updated {', '.join(changed)})")
    else:
        print(f"release VERSION {target} ({level}; already set)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
