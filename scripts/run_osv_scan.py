#!/usr/bin/env python3

from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path

_OSV_SCANNER_APPARENT_REPOS = (
    "osv_scanner_linux_x86_64",
    "osv_scanner_macos_x86_64",
    "osv_scanner_macos_arm64",
)


def _repo_root() -> Path:
    ws = os.environ.get("BUILD_WORKSPACE_DIRECTORY")
    if ws:
        return Path(ws)
    return Path(__file__).resolve().parent.parent


def _osv_scanner_canonical_repos(r) -> list[str]:
    mapping_path = r.Rlocation("_repo_mapping")
    if not mapping_path or not os.path.isfile(mapping_path):
        return []
    canonical_repos = []
    with open(mapping_path, encoding="utf-8") as f:
        for line in f:
            parts = line.rstrip("\n").split(",")
            if len(parts) != 3:
                continue
            source_repo, apparent_name, canonical_name = parts
            if source_repo == "" and apparent_name in _OSV_SCANNER_APPARENT_REPOS:
                canonical_repos.append(canonical_name)
    return canonical_repos


def find_osv_scanner_bin() -> str:
    path = os.environ.get("ARENA_OSV_SCANNER_BIN")
    if path and os.path.isfile(path):
        return path

    from bazel_tools.tools.python.runfiles import runfiles

    r = runfiles.Create()
    if r is not None:
        for canonical_repo in _osv_scanner_canonical_repos(r):
            p = r.Rlocation(f"{canonical_repo}/file/downloaded")
            if p and os.path.isfile(p):
                return p

    raise RuntimeError(
        "osv-scanner binary not found; set ARENA_OSV_SCANNER_BIN or run via "
        "`bazel run //scripts:run_osv_scan`"
    )


def main() -> int:
    root = _repo_root()
    osv_scanner_bin = find_osv_scanner_bin()
    config_path = root / "osv-scanner.toml"
    print(f"running osv-scanner against {root} (config: {config_path})")
    result = subprocess.run(
        [
            osv_scanner_bin,
            "scan",
            "source",
            "--recursive",
            "--config",
            str(config_path),
            str(root),
        ]
    )
    return result.returncode


if __name__ == "__main__":
    sys.exit(main())
