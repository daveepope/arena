#!/usr/bin/env python3

from __future__ import annotations

import os
import sys
from pathlib import Path

from arena_version import (
    read_version,
    read_version_from_git_ref,
    release_version_increased,
    release_version_only,
)


def _repo_root() -> Path:
    ws = os.environ.get("BUILD_WORKSPACE_DIRECTORY")
    if ws:
        return Path(ws)
    return Path(__file__).resolve().parent.parent


def validate_version_bump(root: Path, base_ref: str) -> tuple[int, str]:
    head = release_version_only(read_version(root))
    try:
        base = read_version_from_git_ref(root, base_ref)
    except ValueError as exc:
        return 1, f"could not read base version at {base_ref}: {exc}"
    if head == base:
        return (
            1,
            f"publishable changes require a VERSION bump (still {head}); "
            "bump VERSION, then run: bazel run //scripts:sync_version",
        )
    if not release_version_increased(base, head):
        return 1, f"VERSION must increase ({base} -> {head})"
    return 0, f"VERSION bumped: {base} -> {head}"


def main() -> int:
    root = _repo_root()
    base_ref = os.environ.get("ARENA_VERSION_BASE_REF", "origin/master").strip()
    code, message = validate_version_bump(root, base_ref)
    if code != 0:
        print(f"error: {message}", file=sys.stderr)
        return code
    print(message)
    return 0


if __name__ == "__main__":
    sys.exit(main())
