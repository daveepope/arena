#!/usr/bin/env python3

from __future__ import annotations

import os
import sys
from pathlib import Path

from arena_version import (
    read_version,
    release_lockfiles_need_repin,
    release_version_only,
)


def _repo_root() -> Path:
    ws = os.environ.get("BUILD_WORKSPACE_DIRECTORY")
    if ws:
        return Path(ws)
    return Path(__file__).resolve().parent.parent


def release_lockfiles_match_version(root: Path) -> tuple[bool, str]:
    target = release_version_only(read_version(root))
    if release_lockfiles_need_repin(root, target):
        return (
            False,
            f"Cargo.Bazel.lock workspace version does not match VERSION ({target})",
        )
    return True, f"release lockfiles match VERSION ({target})"


def main() -> int:
    root = _repo_root()
    ok, message = release_lockfiles_match_version(root)
    if ok:
        print(message)
        return 0
    print(f"error: {message}", file=sys.stderr)
    print(
        "Run: CARGO_BAZEL_REPIN=1 bazel build //... && bazel mod deps --lockfile_mode=update",
        file=sys.stderr,
    )
    return 1


if __name__ == "__main__":
    sys.exit(main())
