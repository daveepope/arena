#!/usr/bin/env python3

from __future__ import annotations

import os
import sys
from pathlib import Path

from arena_version import read_version, sync_workspace_version


def _repo_root() -> Path:
    ws = os.environ.get("BUILD_WORKSPACE_DIRECTORY")
    if ws:
        return Path(ws)
    return Path(__file__).resolve().parent.parent


def main() -> int:
    root = _repo_root()
    version = read_version(root)
    changed = sync_workspace_version(root)
    if changed:
        for name in changed:
            print(f"updated {name} -> {version}")
        print("if Rust deps changed, run: CARGO_BAZEL_REPIN=1 bazel build //...")
    else:
        print(f"already in sync at {version}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
