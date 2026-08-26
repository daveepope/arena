#!/usr/bin/env python3

from __future__ import annotations

import os
import sys
from pathlib import Path

from arena_version import (
    audit_arena_ffi_binary,
    check_cargo_vet_watermarks,
    run_cargo_vet_check_report,
    vet_rust_dependencies,
)


def _repo_root() -> Path:
    ws = os.environ.get("BUILD_WORKSPACE_DIRECTORY")
    if ws:
        return Path(ws)
    return Path(__file__).resolve().parent.parent


def main() -> int:
    root = _repo_root()
    audit_arena_ffi_binary(root)
    vet_rust_dependencies(root)
    check_cargo_vet_watermarks(run_cargo_vet_check_report(root))
    return 0


if __name__ == "__main__":
    sys.exit(main())
