#!/usr/bin/env python3

from __future__ import annotations

import os
import sys
from pathlib import Path

from arena_version import (
    require_cargo,
    audit_arena_ffi_binary,
    record_bazel_lockfiles,
    regenerate_windows_pip_locks,
    repin_all_lockfiles,
    vet_rust_dependencies,
)
from run_osv_scan import main as run_osv_scan


def _repo_root() -> Path:
    ws = os.environ.get("BUILD_WORKSPACE_DIRECTORY")
    if ws:
        return Path(ws)
    return Path(__file__).resolve().parent.parent


def main() -> int:
    root = _repo_root()
    require_cargo()
    print("repinning Rust, Python, and Maven lockfiles")
    repin_all_lockfiles(root)
    print(
        "updated Cargo.Bazel.lock, Cargo.lock, MODULE.bazel.lock, "
        "arena_java_maven_install.json, requirements_lock.txt"
    )
    audit_arena_ffi_binary(root)
    vet_rust_dependencies(root)
    print("scanning repinned lockfiles for known CVEs (Rust, JS, Python, Maven, .NET)")
    scan_status = run_osv_scan()
    if scan_status != 0:
        raise RuntimeError(
            f"osv-scanner found vulnerabilities (exit code {scan_status}); see output above"
        )
    regenerate_windows_pip_locks(root)
    record_bazel_lockfiles(root)
    print("recorded Cargo.lock and MODULE.bazel.lock against the final Bazel resolution")
    return 0


if __name__ == "__main__":
    sys.exit(main())
