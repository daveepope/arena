#!/usr/bin/env python3

from __future__ import annotations

import os
import sys
from pathlib import Path

from arena_version import (
    prepare_preview_from_base,
    prepare_release_from_base,
    preview_only_from_env,
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
    if preview_only_from_env():
        target, changed = prepare_preview_from_base(root, base_ref, level)
        label = "preview for TestPyPI"
    else:
        target, changed = prepare_release_from_base(root, base_ref, level)
        label = "release"
    if changed:
        print(f"release VERSION {target} ({level}; {label}; updated {', '.join(changed)})")
    else:
        print(f"release VERSION {target} ({level}; {label})")
    github_output = os.environ.get("GITHUB_OUTPUT")
    if github_output:
        with open(github_output, "a", encoding="utf-8") as out:
            out.write(f"release_target={target}\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
