#!/usr/bin/env python3

from __future__ import annotations

import os
import sys
from pathlib import Path

from arena_version import (
    bump_patch_version,
    read_version,
    read_version_from_git_ref,
    release_lockfiles_need_repin,
    release_version_increased,
    release_version_only,
    repin_release_lockfiles,
    sync_workspace_version,
    write_version,
)


def _repo_root() -> Path:
    ws = os.environ.get("BUILD_WORKSPACE_DIRECTORY")
    if ws:
        return Path(ws)
    return Path(__file__).resolve().parent.parent


def bump_patch_release_from_base(root: Path, base_ref: str) -> tuple[str, list[str]]:
    base = release_version_only(read_version_from_git_ref(root, base_ref))
    target = bump_patch_version(base)
    head = release_version_only(read_version(root))
    if not release_version_increased(base, head):
        write_version(root, target)
        head = target
    changed = sync_workspace_version(root)
    return head, changed


def main() -> int:
    root = _repo_root()
    base_ref = os.environ.get("ARENA_VERSION_BASE_REF", "origin/master").strip()
    head, changed = bump_patch_release_from_base(root, base_ref)
    print(f"VERSION {head}")
    for name in changed:
        print(f"synced {name}")
    if changed or release_lockfiles_need_repin(root, head):
        print("repinning release lockfiles")
        repin_release_lockfiles(root)
        print("updated Cargo.Bazel.lock, Cargo.lock, MODULE.bazel.lock")
    return 0


if __name__ == "__main__":
    sys.exit(main())
