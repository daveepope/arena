#!/usr/bin/env python3

from __future__ import annotations

import os
import re
import sys
from pathlib import Path


def _repo_root() -> Path:
    ws = os.environ.get("BUILD_WORKSPACE_DIRECTORY")
    if ws:
        return Path(ws)
    return Path(__file__).resolve().parent.parent.parent


def pinned_constant(pinned_versions_text: str, constant_name: str) -> str:
    match = re.search(rf'^{constant_name} = "([^"]+)"$', pinned_versions_text, re.MULTILINE)
    if not match:
        raise ValueError(f"could not find {constant_name} in pinned_dependency_versions.bzl")
    return match.group(1)


def nuget_archive_version(module_bazel_text: str, repo_name: str) -> str:
    match = re.search(
        rf'nuget_archive\(\s*name = "{repo_name}",.*?version = "([^"]+)",\s*\)',
        module_bazel_text,
        re.DOTALL,
    )
    if not match:
        raise ValueError(f"could not find nuget_archive(name = \"{repo_name}\", ...) in MODULE.bazel")
    return match.group(1)


def check_pinned_versions_match(root: Path) -> list[str]:
    module_bazel_text = (root / "MODULE.bazel").read_text(encoding="utf-8")
    pinned_versions_text = (root / "tools/dotnet_nuget/pinned_dependency_versions.bzl").read_text(encoding="utf-8")

    mismatches = []
    for repo_name, constant_name in (
        ("newtonsoft_json", "NEWTONSOFT_JSON_VERSION"),
        ("ms_logging_abstractions", "MS_LOGGING_ABSTRACTIONS_VERSION"),
    ):
        archive_version = nuget_archive_version(module_bazel_text, repo_name)
        pinned_version = pinned_constant(pinned_versions_text, constant_name)
        if archive_version != pinned_version:
            mismatches.append(
                f"MODULE.bazel's nuget_archive(name = \"{repo_name}\") pins {archive_version!r}, "
                f"but pinned_dependency_versions.bzl's {constant_name} is {pinned_version!r} "
                "(arena-xunit's published .nuspec dependency metadata uses the latter - they must match)"
            )
    return mismatches


def main() -> int:
    mismatches = check_pinned_versions_match(_repo_root())
    for mismatch in mismatches:
        print(mismatch, file=sys.stderr)
    return 1 if mismatches else 0


if __name__ == "__main__":
    sys.exit(main())
