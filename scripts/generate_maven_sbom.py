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


def build_sbom(maven_install_lock: dict) -> dict:
    components = []
    for coordinates, entry in maven_install_lock.get("artifacts", {}).items():
        group, artifact = coordinates.split(":", 1)
        version = entry.get("version")
        if not version:
            continue
        components.append(
            {
                "type": "library",
                "name": coordinates,
                "version": version,
                "purl": f"pkg:maven/{group}/{artifact}@{version}",
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
        description="Generate a CycloneDX SBOM for arena_java_maven from arena_java_maven_install.json."
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
    lock_text = (root / "arena_java_maven_install.json").read_text(encoding="utf-8")
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
