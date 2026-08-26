#!/usr/bin/env python3

from __future__ import annotations

import argparse
import json
import os
import sys
from pathlib import Path


def _repo_root() -> Path:
    ws = os.environ.get("BUILD_WORKSPACE_DIRECTORY")
    if ws:
        return Path(ws)
    return Path(__file__).resolve().parent.parent


def _local_crate_names(cargo_bazel_lock: dict) -> set[str]:
    names: set[str] = set()
    for entry in cargo_bazel_lock.get("crates", {}).values():
        if entry.get("package_url") is None:
            name = entry.get("name")
            if name:
                names.add(name)
    return names


def build_sbom(cargo_bazel_lock: dict) -> dict:
    local_names = _local_crate_names(cargo_bazel_lock)
    components = []
    for entry in cargo_bazel_lock.get("crates", {}).values():
        name = entry.get("name")
        version = entry.get("version")
        if not name or not version or name in local_names:
            continue
        components.append(
            {
                "type": "library",
                "name": name,
                "version": version,
                "purl": f"pkg:cargo/{name}@{version}",
            }
        )
    components.sort(key=lambda c: (c["name"], c["version"]))
    return {
        "bomFormat": "CycloneDX",
        "specVersion": "1.5",
        "version": 1,
        "components": components,
    }


def _parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Generate a CycloneDX SBOM for arena_ffi_shared from Cargo.Bazel.lock."
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=None,
        help="path to write the SBOM JSON to (default: stdout)",
    )
    return parser.parse_args(argv)


def main() -> int:
    args = _parse_args(sys.argv[1:])
    root = _repo_root()
    lock_text = (root / "Cargo.Bazel.lock").read_text(encoding="utf-8")
    sbom = build_sbom(json.loads(lock_text))
    output_text = json.dumps(sbom, indent=2) + "\n"
    if args.output:
        args.output.write_text(output_text, encoding="utf-8")
        print(f"wrote SBOM ({len(sbom['components'])} components) to {args.output}")
    else:
        print(output_text, end="")
    return 0


if __name__ == "__main__":
    sys.exit(main())
