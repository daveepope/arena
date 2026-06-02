from __future__ import annotations

import json
import os
import re
import subprocess
from pathlib import Path


def read_version(root: Path) -> str:
    version = (root / "VERSION").read_text(encoding="utf-8").strip()
    if not version:
        raise ValueError(f"{root / 'VERSION'} is empty")
    return version


def cargo_workspace_version(root: Path) -> str | None:
    cargo = (root / "Cargo.toml").read_text(encoding="utf-8")
    match = re.search(r'^version = "([^"]+)"\s*$', cargo, re.MULTILINE)
    return match.group(1) if match else None


def module_bazel_version(root: Path) -> str | None:
    module = (root / "MODULE.bazel").read_text(encoding="utf-8")
    match = re.search(
        r'module\(\n    name = "arena",\n    version = "([^"]+)"',
        module,
    )
    return match.group(1) if match else None


def is_synced(root: Path) -> bool:
    version = read_version(root)
    return (
        cargo_workspace_version(root) == version
        and module_bazel_version(root) == version
    )


def sync_workspace_version(root: Path) -> list[str]:
    version = read_version(root)
    changed: list[str] = []

    cargo_path = root / "Cargo.toml"
    cargo = cargo_path.read_text(encoding="utf-8")
    new_cargo, n = re.subn(
        r'^(version = ")[^"]+(")\s*$',
        rf'\g<1>{version}\g<2>',
        cargo,
        count=1,
        flags=re.MULTILINE,
    )
    if n != 1:
        raise RuntimeError(f"could not update [workspace.package] version in {cargo_path}")
    if new_cargo != cargo:
        cargo_path.write_text(new_cargo, encoding="utf-8", newline="\n")
        changed.append("Cargo.toml")

    module_path = root / "MODULE.bazel"
    module = module_path.read_text(encoding="utf-8")
    new_module, n = re.subn(
        r'^(module\(\n    name = "arena",\n    version = ")[^"]+(")',
        rf'\g<1>{version}\g<2>',
        module,
        count=1,
    )
    if n != 1:
        raise RuntimeError(f"could not update module() version in {module_path}")
    if new_module != module:
        module_path.write_text(new_module, encoding="utf-8", newline="\n")
        changed.append("MODULE.bazel")

    return changed


def parse_release_version(version: str) -> tuple[int, int, int]:
    match = re.fullmatch(r"(\d+)\.(\d+)\.(\d+)", version.strip())
    if not match:
        raise ValueError(f"VERSION must be MAJOR.MINOR.PATCH, got {version!r}")
    return int(match.group(1)), int(match.group(2)), int(match.group(3))


def bump_release_version(version: str, level: str) -> str:
    major, minor, patch = parse_release_version(version)
    if level == "major":
        return f"{major + 1}.0.0"
    if level == "minor":
        return f"{major}.{minor + 1}.0"
    if level == "patch":
        return f"{major}.{minor}.{patch + 1}"
    raise ValueError(f"unsupported semver level {level!r}")


def resolve_semver_level_from_event(event_path: Path) -> str:
    payload = json.loads(event_path.read_text(encoding="utf-8"))
    labels = [
        label["name"]
        for label in payload.get("pull_request", {}).get("labels", [])
    ]
    if "semver:major" in labels:
        return "major"
    if "semver:minor" in labels:
        return "minor"
    return "patch"


def read_version_from_git_ref(root: Path, ref: str) -> str:
    result = subprocess.run(
        ["git", "show", f"{ref}:VERSION"],
        cwd=root,
        check=True,
        capture_output=True,
        text=True,
    )
    version = result.stdout.strip()
    if not version:
        raise ValueError(f"VERSION is empty at {ref}")
    return version


def prepare_release_version(root: Path, master_ref: str, level: str) -> tuple[str, list[str]]:
    base = read_version_from_git_ref(root, master_ref)
    target = bump_release_version(base, level)
    version_path = root / "VERSION"
    changed: list[str] = []
    if version_path.read_text(encoding="utf-8").strip() != target:
        version_path.write_text(target + "\n", encoding="utf-8")
        changed.append("VERSION")
    changed.extend(sync_workspace_version(root))
    return target, changed


def resolve_semver_level_from_env() -> str:
    event_path = os.environ.get("GITHUB_EVENT_PATH")
    if event_path:
        return resolve_semver_level_from_event(Path(event_path))
    level = os.environ.get("ARENA_SEMVER_LEVEL", "patch").strip().lower()
    if level in ("major", "minor", "patch"):
        return level
    raise ValueError(f"unsupported ARENA_SEMVER_LEVEL {level!r}")
