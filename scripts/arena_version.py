from __future__ import annotations

import json
import os
import re
import shutil
import subprocess
import sys
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


def higher_release_version(a: str, b: str) -> str:
    return a if parse_release_version(a) >= parse_release_version(b) else b


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


def _cargo_cdylib_name(crate_name: str) -> str:
    if sys.platform == "win32":
        return f"{crate_name}.dll"
    if sys.platform == "darwin":
        return f"lib{crate_name}.dylib"
    return f"lib{crate_name}.so"


def regenerate_windows_pip_locks(root: Path) -> None:
    if sys.platform != "win32":
        print(
            "skipping requirements_windows.txt regeneration: not running on native "
            "Windows (WSL/Linux/macOS produce incorrect Windows wheel hashes). Run "
            "this from native Windows, or dispatch the 'Generate Windows pip lock "
            "files' GitHub Actions workflow."
        )
        return
    bazel = os.environ.get("BAZEL", "bazel")
    env = os.environ.copy()
    for target in ["//arena-pytest:pip_requirements.update", "//examples:pip_requirements.update"]:
        subprocess.run([bazel, "run", target], cwd=root, env=env, check=True)


CARGO_AUDITABLE_VERSION = "0.7.5"
CARGO_AUDIT_VERSION = "0.22.2"
CARGO_VET_VERSION = "0.10.2"


def audit_arena_ffi_binary(root: Path) -> None:
    env = os.environ.copy()
    if shutil.which("cargo-auditable") is None or shutil.which("cargo-audit") is None:
        subprocess.run(
            [
                "cargo",
                "install",
                f"cargo-auditable@{CARGO_AUDITABLE_VERSION}",
                f"cargo-audit@{CARGO_AUDIT_VERSION}",
                "--locked",
            ],
            cwd=root,
            env=env,
            check=True,
        )
    subprocess.run(
        ["cargo", "auditable", "build", "--release", "--lib", "-p", "arena-ffi"],
        cwd=root,
        env=env,
        check=True,
    )
    binary = root / "target" / "release" / _cargo_cdylib_name("arena_ffi")
    subprocess.run(["cargo", "audit", "bin", str(binary)], cwd=root, env=env, check=True)


def vet_rust_dependencies(root: Path) -> None:
    env = os.environ.copy()
    if shutil.which("cargo-vet") is None:
        subprocess.run(
            ["cargo", "install", f"cargo-vet@{CARGO_VET_VERSION}", "--locked"],
            cwd=root,
            env=env,
            check=True,
        )
    subprocess.run(["cargo", "vet"], cwd=root, env=env, check=True)


def run_cargo_vet_check_report(root: Path) -> dict:
    env = os.environ.copy()
    result = subprocess.run(
        ["cargo", "vet", "check", "--output-format=json"],
        cwd=root,
        env=env,
        check=True,
        capture_output=True,
        text=True,
    )
    return json.loads(result.stdout)


CARGO_VET_AUDITED_PACKAGE_COUNT_WATERMARK = 0
CARGO_VET_EXEMPTED_PACKAGE_COUNT_WATERMARK = 479


def _check_cargo_vet_watermark(
    count: int,
    watermark: int,
    constant: str,
    grew_explanation: str,
    shrank_explanation: str,
) -> None:
    if count == watermark:
        return
    explanation = grew_explanation if count > watermark else shrank_explanation
    raise SystemExit(
        f"cargo-vet watermark mismatch: {constant} expects {watermark}, found {count}.\n"
        f"{explanation}\n"
        f"If this is intentional, update {constant} in scripts/arena_version.py to {count}."
    )


def check_cargo_vet_watermarks(report: dict) -> None:
    audited = len(report.get("vetted_fully", [])) + len(report.get("vetted_partially", []))
    exempted = len(report.get("vetted_with_exemptions", []))

    _check_cargo_vet_watermark(
        exempted,
        CARGO_VET_EXEMPTED_PACKAGE_COUNT_WATERMARK,
        "CARGO_VET_EXEMPTED_PACKAGE_COUNT_WATERMARK",
        grew_explanation=(
            "This means a dependency is now trusted only through a self-certified exemption "
            "in supply-chain/config.toml, not a real audit (likely a new or updated "
            "dependency was added and `cargo vet regenerate exemptions` or "
            "`cargo vet add-exemption` was run to make `cargo vet check` pass). Run "
            "`cargo vet suggest` to see what's unaudited, and prefer `cargo vet certify` "
            "over accepting another exemption."
        ),
        shrank_explanation=(
            "This is an improvement: a dependency that used to rely on a self-certified "
            "exemption was either removed, or its exemption in supply-chain/config.toml was "
            "replaced by a real audit in supply-chain/audits.toml."
        ),
    )
    _check_cargo_vet_watermark(
        audited,
        CARGO_VET_AUDITED_PACKAGE_COUNT_WATERMARK,
        "CARGO_VET_AUDITED_PACKAGE_COUNT_WATERMARK",
        grew_explanation=(
            "This is an improvement: someone ran `cargo vet certify` and "
            "supply-chain/audits.toml now covers a package that previously relied on an "
            "exemption."
        ),
        shrank_explanation=(
            "A previously certified audit in supply-chain/audits.toml is gone, or the "
            "package it covered was removed. Investigate before accepting this."
        ),
    )


def repin_all_lockfiles(root: Path) -> None:
    bazel = os.environ.get("BAZEL", "bazel")
    env = os.environ.copy()
    env["CARGO_BAZEL_REPIN"] = "1"
    bazel_config = os.environ.get("ARENA_BAZEL_CONFIG", "").strip()

    commands = [
        [bazel, "build", "//..."],
        [bazel, "mod", "deps", "--lockfile_mode=update"],
        [bazel, "run", "@arena_java_maven//:pin"],
        [bazel, "run", "//arena-pytest:pip_requirements.update"],
        [bazel, "run", "//examples:pip_requirements.update"],
    ]
    for args in commands:
        if bazel_config:
            args.append(f"--config={bazel_config}")
        subprocess.run(args, cwd=root, env=env, check=True)
