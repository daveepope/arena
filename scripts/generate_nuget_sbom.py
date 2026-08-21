#!/usr/bin/env python3

from __future__ import annotations

import argparse
import json
import os
import re
import sys
from pathlib import Path


def _repo_root() -> Path:
    ws = os.environ.get("BUILD_WORKSPACE_DIRECTORY")
    if ws:
        return Path(ws)
    return Path(__file__).resolve().parent.parent


def parse_nuget_archives(module_bazel_text: str) -> set[tuple[str, str]]:
    pairs: set[tuple[str, str]] = set()
    for match in re.finditer(r"nuget_archive\(([^)]*)\)", module_bazel_text, re.DOTALL):
        body = match.group(1)
        id_match = re.search(r'id\s*=\s*"([^"]+)"', body)
        version_match = re.search(r'version\s*=\s*"([^"]+)"', body)
        if id_match and version_match:
            pairs.add((id_match.group(1), version_match.group(1)))
    return pairs


def build_sbom(module_bazel_text: str) -> dict:
    components = [
        {
            "type": "library",
            "name": package_id,
            "version": version,
            "purl": f"pkg:nuget/{package_id}@{version}",
        }
        for package_id, version in parse_nuget_archives(module_bazel_text)
    ]
    components.sort(key=lambda c: (c["name"], c["version"]))
    return {
        "bomFormat": "CycloneDX",
        "specVersion": "1.5",
        "version": 1,
        "components": components,
    }


def _parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Generate a CycloneDX SBOM for arena-xunit's NuGet deps from MODULE.bazel."
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
    module_text = (root / "MODULE.bazel").read_text(encoding="utf-8")
    sbom = build_sbom(module_text)
    output_text = json.dumps(sbom, indent=2) + "\n"
    if args.output:
        args.output.write_text(output_text, encoding="utf-8")
        print(f"wrote SBOM ({len(sbom['components'])} components) to {args.output}")
    else:
        print(output_text, end="")
    return 0


if __name__ == "__main__":
    sys.exit(main())
