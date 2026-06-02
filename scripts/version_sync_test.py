#!/usr/bin/env python3

from __future__ import annotations

import os
import sys
from pathlib import Path

from arena_version import is_synced, read_version, sync_workspace_version


def _repo_root() -> Path:
    ws = os.environ.get("BUILD_WORKSPACE_DIRECTORY")
    if ws:
        return Path(ws)
    return Path(__file__).resolve().parent.parent


def main() -> int:
    root = _repo_root()
    version = read_version(root)
    if is_synced(root):
        return 0

    changed = sync_workspace_version(root)
    print(
        f"error: VERSION is {version} but Cargo.toml/MODULE.bazel were out of date; "
        f"auto-sync updated: {', '.join(changed)}. "
        "Commit those files (and repin Cargo.Bazel.lock if needed).",
        file=sys.stderr,
    )
    return 1


if __name__ == "__main__":
    sys.exit(main())
