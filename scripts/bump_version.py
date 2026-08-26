#!/usr/bin/env python3

from __future__ import annotations

import argparse
import os
import sys
from pathlib import Path

from arena_version import (
    bump_version,
    higher_release_version,
    read_version,
    read_version_from_git_ref,
    release_lockfiles_need_repin,
    release_version_increased,
    release_version_only,
    repin_all_lockfiles,
    sync_workspace_version,
    write_version,
)


def _repo_root() -> Path:
    ws = os.environ.get("BUILD_WORKSPACE_DIRECTORY")
    if ws:
        return Path(ws)
    return Path(__file__).resolve().parent.parent


def bump_release_from_base(
    root: Path, base_ref: str, kind: str = "patch"
) -> tuple[str, list[str]]:
    head = release_version_only(read_version(root))
    base = release_version_only(read_version_from_git_ref(root, base_ref))
    if kind == "patch":
        if not release_version_increased(base, head):
            head = bump_version(base, kind)
            write_version(root, head)
    else:
        head = bump_version(higher_release_version(head, base), kind)
        write_version(root, head)
    changed = sync_workspace_version(root)
    return head, changed


def _parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Bump the Arena release version.")
    parser.add_argument(
        "--bump",
        choices=["major", "minor", "patch"],
        default="patch",
        help="which part of MAJOR.MINOR.PATCH to bump (default: patch)",
    )
    return parser.parse_args(argv)


def main() -> int:
    args = _parse_args(sys.argv[1:])
    root = _repo_root()
    base_ref = os.environ.get("ARENA_VERSION_BASE_REF", "origin/master").strip()
    head, changed = bump_release_from_base(root, base_ref, args.bump)
    print(f"VERSION {head}")
    for name in changed:
        print(f"synced {name}")
    if changed or release_lockfiles_need_repin(root, head):
        print("repinning release lockfiles")
        repin_all_lockfiles(root)
        print(
            "updated Cargo.Bazel.lock, Cargo.lock, MODULE.bazel.lock, "
            "arena_java_maven_install.json, requirements_lock.txt"
        )
    return 0


if __name__ == "__main__":
    sys.exit(main())
