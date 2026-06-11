#!/usr/bin/env python3

from __future__ import annotations

import os
import sys
from pathlib import Path

from arena_version import (
    prepare_release_from_base,
    release_lockfiles_need_repin,
    resolve_semver_level_from_env,
)


def _repo_root() -> Path:
    ws = os.environ.get("BUILD_WORKSPACE_DIRECTORY")
    if ws:
        return Path(ws)
    return Path(__file__).resolve().parent.parent


def main() -> int:
    root = _repo_root()
    level = resolve_semver_level_from_env()
    base_ref = os.environ.get("ARENA_VERSION_BASE_REF", "origin/master")
    target, changed = prepare_release_from_base(root, base_ref, level)
    needs_repin = release_lockfiles_need_repin(root, target)
    if changed:
        print(f"release VERSION {target} ({level}; updated {', '.join(changed)})")
    else:
        print(f"release VERSION {target} ({level}; preview)")
    if needs_repin:
        print(f"lockfiles need repin for workspace version {target}")
    github_output = os.environ.get("GITHUB_OUTPUT")
    if github_output:
        with open(github_output, "a", encoding="utf-8") as out:
            out.write(f"release_target={target}\n")
            out.write(f"needs_repin={'true' if needs_repin else 'false'}\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
