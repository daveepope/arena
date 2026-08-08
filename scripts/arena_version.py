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


def write_version(root: Path, version: str) -> None:
    (root / "VERSION").write_text(version.strip() + "\n", encoding="utf-8", newline="\n")


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


def release_version_only(version: str) -> str:
    match = re.match(r"(\d+\.\d+\.\d+)", version.strip())
    if not match:
        raise ValueError(f"VERSION must start with MAJOR.MINOR.PATCH, got {version!r}")
    return match.group(1)


def pyproject_uses_dynamic_version(root: Path) -> bool:
    pyproject = (root / "arena-pytest/pyproject.toml").read_text(encoding="utf-8")
    return 'dynamic = ["version"]' in pyproject


def pyproject_version(root: Path) -> str | None:
    if pyproject_uses_dynamic_version(root):
        return read_version(root)
    pyproject = (root / "arena-pytest/pyproject.toml").read_text(encoding="utf-8")
    match = re.search(r'^version = "([^"]+)"', pyproject, re.MULTILINE)
    return match.group(1) if match else None


def is_synced(root: Path) -> bool:
    version = read_version(root)
    return (
        cargo_workspace_version(root) == version
        and module_bazel_version(root) == version
        and pyproject_version(root) == version
    )


def sync_workspace_version(root: Path) -> list[str]:
    version = read_version(root)
    changed: list[str] = []

    cargo_path = root / "Cargo.toml"
    if cargo_workspace_version(root) != version:
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
        cargo_path.write_text(new_cargo, encoding="utf-8", newline="\n")
        changed.append("Cargo.toml")

    module_path = root / "MODULE.bazel"
    if module_bazel_version(root) != version:
        module = module_path.read_text(encoding="utf-8")
        new_module, n = re.subn(
            r'^(module\(\n    name = "arena",\n    version = ")[^"]+(")',
            rf'\g<1>{version}\g<2>',
            module,
            count=1,
        )
        if n != 1:
            raise RuntimeError(f"could not update module() version in {module_path}")
        module_path.write_text(new_module, encoding="utf-8", newline="\n")
        changed.append("MODULE.bazel")

    return changed


def parse_release_version(version: str) -> tuple[int, int, int]:
    match = re.fullmatch(r"(\d+)\.(\d+)\.(\d+)", release_version_only(version))
    if not match:
        raise ValueError(f"VERSION must be MAJOR.MINOR.PATCH, got {version!r}")
    return int(match.group(1)), int(match.group(2)), int(match.group(3))


def bump_patch_version(version: str) -> str:
    major, minor, patch = parse_release_version(version)
    return f"{major}.{minor}.{patch + 1}"


def bump_minor_version(version: str) -> str:
    major, minor, _patch = parse_release_version(version)
    return f"{major}.{minor + 1}.0"


def bump_major_version(version: str) -> str:
    major, _minor, _patch = parse_release_version(version)
    return f"{major + 1}.0.0"


_BUMP_KIND_FNS = {
    "major": bump_major_version,
    "minor": bump_minor_version,
    "patch": bump_patch_version,
}


def bump_version(version: str, kind: str) -> str:
    try:
        fn = _BUMP_KIND_FNS[kind]
    except KeyError:
        raise ValueError(
            f"unknown bump kind {kind!r}; expected one of {sorted(_BUMP_KIND_FNS)}"
        ) from None
    return fn(version)


def release_version_increased(base: str, head: str) -> bool:
    return parse_release_version(head) > parse_release_version(base)


def _git_show_at_ref(root: Path, ref: str, path: str) -> str | None:
    result = subprocess.run(
        ["git", "show", f"{ref}:{path}"],
        cwd=root,
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        return None
    return result.stdout


def _parse_cargo_workspace_version_text(cargo: str) -> str | None:
    match = re.search(r'^version = "([^"]+)"\s*$', cargo, re.MULTILINE)
    if not match:
        return None
    return release_version_only(match.group(1))


def _parse_module_bazel_version_text(module: str) -> str | None:
    match = re.search(
        r'module\(\n    name = "arena",\n    version = "([^"]+)"',
        module,
    )
    if not match:
        return None
    return release_version_only(match.group(1))


def read_version_from_git_ref(root: Path, ref: str) -> str:
    version_text = _git_show_at_ref(root, ref, "VERSION")
    if version_text is not None:
        version = version_text.strip()
        if version:
            return release_version_only(version)

    cargo_text = _git_show_at_ref(root, ref, "Cargo.toml")
    if cargo_text is not None:
        parsed = _parse_cargo_workspace_version_text(cargo_text)
        if parsed:
            return parsed

    module_text = _git_show_at_ref(root, ref, "MODULE.bazel")
    if module_text is not None:
        parsed = _parse_module_bazel_version_text(module_text)
        if parsed:
            return parsed

    raise ValueError(
        f"no release version found at {ref} (expected VERSION, Cargo.toml, or MODULE.bazel)"
    )


def workspace_version_in_cargo_bazel_lock(root: Path) -> str | None:
    lock_path = root / "Cargo.Bazel.lock"
    if not lock_path.exists():
        return None
    data = json.loads(lock_path.read_text(encoding="utf-8"))
    for entry in data.get("crates", {}).values():
        if entry.get("name") != "arena":
            continue
        if entry.get("package_url") is not None:
            continue
        version = entry.get("version")
        if version:
            return release_version_only(version)
    return None


def release_lockfiles_need_repin(root: Path, target: str) -> bool:
    locked = workspace_version_in_cargo_bazel_lock(root)
    if locked is None:
        cargo = cargo_workspace_version(root)
        if cargo is None:
            return True
        return release_version_only(cargo) != target
    return locked != target


def repin_release_lockfiles(root: Path) -> None:
    bazel = os.environ.get("BAZEL", "bazel")
    env = os.environ.copy()
    env["CARGO_BAZEL_REPIN"] = "1"
    build_args = [bazel, "build", "//..."]
    mod_args = [bazel, "mod", "deps", "--lockfile_mode=update"]
    bazel_config = os.environ.get("ARENA_BAZEL_CONFIG", "").strip()
    if bazel_config:
        build_args.append(f"--config={bazel_config}")
        mod_args.append(f"--config={bazel_config}")
    subprocess.run(build_args, cwd=root, env=env, check=True)
    subprocess.run(mod_args, cwd=root, env=env, check=True)
