#!/usr/bin/env python3

from __future__ import annotations

import os
import sys
from pathlib import Path

from arena_version import audit_arena_ffi_binary, vet_rust_dependencies


def _repo_root() -> Path:
    ws = os.environ.get("BUILD_WORKSPACE_DIRECTORY")
    if ws:
        return Path(ws)
    return Path(__file__).resolve().parent.parent


def main() -> int:
    root = _repo_root()
    audit_arena_ffi_binary(root)
    vet_rust_dependencies(root)
    return 0


if __name__ == "__main__":
    sys.exit(main())
