#!/usr/bin/env python3

from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path

from arena_version import (
    read_version,
    release_lockfiles_need_repin,
    release_version_only,
    repin_release_lockfiles,
    sync_workspace_version,
)


def _repo_root() -> Path:
    ws = os.environ.get("BUILD_WORKSPACE_DIRECTORY")
    if ws:
        return Path(ws)
    return Path(__file__).resolve().parent.parent


def main() -> int:
    root = _repo_root()
    version = release_version_only(read_version(root))
    changed = sync_workspace_version(root)
    if changed:
        for name in changed:
            print(f"updated {name} -> {version}")
    elif release_lockfiles_need_repin(root, version):
        print(f"VERSION files already at {version}")
    else:
        print(f"already in sync at {version}")
        return 0

    if changed or release_lockfiles_need_repin(root, version):
        print("repinning release lockfiles")
        repin_release_lockfiles(root)
        print("updated Cargo.Bazel.lock, Cargo.lock, MODULE.bazel.lock")
    return 0


if __name__ == "__main__":
    sys.exit(main())
