#!/usr/bin/env python3

from __future__ import annotations

import json
import os
import re
import subprocess
import sys
import urllib.error
import urllib.request
from datetime import datetime, timedelta, timezone
from email.utils import parsedate_to_datetime
from pathlib import Path
from typing import Callable

WATCHED_FILES: dict[str, str] = {
    "Cargo.Bazel.lock": "cargo",
    "arena-pytest/requirements_lock.txt": "pip",
    "examples/requirements_lock.txt": "pip",
    "MODULE.bazel": "module",
}

USER_AGENT = "arena-dependency-release-age-check"


def _repo_root() -> Path:
    ws = os.environ.get("BUILD_WORKSPACE_DIRECTORY")
    if ws:
        return Path(ws)
    return Path(__file__).resolve().parent.parent


def _minimum_age() -> timedelta:
    raw = os.environ.get("ARENA_MIN_RELEASE_AGE_DAYS", "3").strip()
    return timedelta(days=int(raw))


def _resolve_base_ref(root: Path) -> str | None:
    env_ref = os.environ.get("ARENA_DEP_AGE_BASE_REF", "").strip()
    if env_ref and env_ref != "0000000000000000000000000000000000000000":
        return env_ref
    for args in (
        ["git", "merge-base", "HEAD", "origin/master"],
        ["git", "merge-base", "HEAD", "master"],
        ["git", "rev-parse", "HEAD~1"],
    ):
        result = subprocess.run(
            args,
            cwd=root,
            capture_output=True,
            text=True,
            check=False,
        )
        if result.returncode == 0:
            ref = result.stdout.strip()
            if ref:
                return ref
    return None


def _git_show(root: Path, ref: str, path: str) -> str | None:
    result = subprocess.run(
        ["git", "show", f"{ref}:{path}"],
        cwd=root,
        capture_output=True,
        text=True,
        check=False,
    )
    if result.returncode != 0:
        return None
    return result.stdout


def _changed_watched_files(root: Path, base_ref: str | None) -> list[str]:
    if base_ref is None:
        return list(WATCHED_FILES.keys())
    result = subprocess.run(
        ["git", "diff", "--name-only", base_ref, "HEAD", "--", *WATCHED_FILES.keys()],
        cwd=root,
        capture_output=True,
        text=True,
        check=False,
    )
    if result.returncode != 0:
        return list(WATCHED_FILES.keys())
    return [line.strip() for line in result.stdout.splitlines() if line.strip()]


def local_cargo_crates_from_bazel_lock(text: str) -> set[tuple[str, str]]:
    if not text:
        return set()
    data = json.loads(text)
    local: set[tuple[str, str]] = set()
    for entry in data.get("crates", {}).values():
        if entry.get("package_url") is not None:
            continue
        name = entry.get("name")
        version = entry.get("version")
        if name and version:
            local.add((name, version))
    return local


def parse_cargo_bazel_lock(text: str) -> set[tuple[str, str, str]]:
    data = json.loads(text)
    pairs: set[tuple[str, str, str]] = set()
    for key in data.get("crates", {}):
        idx = key.rfind(" ")
        if idx == -1:
            continue
        name = key[:idx]
        version = key[idx + 1 :]
        pairs.add(("cargo", name, version))
    return pairs


def parse_requirements_lock(text: str) -> set[tuple[str, str, str]]:
    pairs: set[tuple[str, str, str]] = set()
    for line in text.splitlines():
        stripped = line.strip()
        if not stripped or stripped.startswith("#") or "==" not in stripped:
            continue
        left, right = stripped.split("==", 1)
        name = left.strip().split()[0].lower()
        version = right.strip().split()[0]
        pairs.add(("pip", name, version))
    return pairs


def parse_module_bazel(text: str) -> set[tuple[str, str, str]]:
    pairs: set[tuple[str, str, str]] = set()
    for match in re.finditer(r'bazel_dep\(name = "([^"]+)", version = "([^"]+)"\)', text):
        pairs.add(("bcr", match.group(1), match.group(2)))
    for match in re.finditer(r"nuget_archive\(([^)]*)\)", text, re.DOTALL):
        body = match.group(1)
        id_match = re.search(r'id\s*=\s*"([^"]+)"', body)
        version_match = re.search(r'version\s*=\s*"([^"]+)"', body)
        if id_match and version_match:
            pairs.add(("nuget", id_match.group(1), version_match.group(1)))
    for match in re.finditer(r'"([^:"]+):([^:"]+):([^"]+)"', text):
        group, artifact, version = match.group(1), match.group(2), match.group(3)
        if ":" in version:
            continue
        pairs.add(("maven", f"{group}:{artifact}", version))
    return pairs


PARSERS: dict[str, Callable[[str], set[tuple[str, str, str]]]] = {
    "cargo": parse_cargo_bazel_lock,
    "pip": parse_requirements_lock,
    "module": parse_module_bazel,
}


def _read_file_at_ref(root: Path, ref: str | None, path: str) -> str:
    if ref is None:
        return (root / path).read_text(encoding="utf-8")
    shown = _git_show(root, ref, path)
    if shown is None:
        return ""
    return shown


def _collect_new_versions(
    root: Path,
    base_ref: str | None,
    changed_paths: list[str],
) -> set[tuple[str, str, str]]:
    new_versions: set[tuple[str, str, str]] = set()
    head_ref = "HEAD"
    for path in changed_paths:
        kind = WATCHED_FILES[path]
        parser = PARSERS[kind]
        old_text = _read_file_at_ref(root, base_ref, path)
        new_text = _read_file_at_ref(root, head_ref, path)
        old_pairs = parser(old_text) if old_text else set()
        new_pairs = parser(new_text) if new_text else set()
        new_versions |= new_pairs - old_pairs
    return new_versions


def _http_json(url: str) -> dict | None:
    request = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
    try:
        with urllib.request.urlopen(request, timeout=30) as response:
            return json.loads(response.read().decode("utf-8"))
    except (urllib.error.URLError, json.JSONDecodeError, TimeoutError):
        return None


def _http_last_modified(url: str) -> datetime | None:
    request = urllib.request.Request(url, method="HEAD", headers={"User-Agent": USER_AGENT})
    try:
        with urllib.request.urlopen(request, timeout=30) as response:
            raw = response.headers.get("Last-Modified")
    except (urllib.error.URLError, TimeoutError):
        return None
    if not raw:
        return None
    try:
        parsed = parsedate_to_datetime(raw)
    except (TypeError, ValueError):
        return None
    if parsed.tzinfo is None:
        parsed = parsed.replace(tzinfo=timezone.utc)
    return parsed.astimezone(timezone.utc)


def _parse_timestamp(raw: str) -> datetime | None:
    normalized = raw.strip()
    if normalized.endswith("Z"):
        normalized = normalized[:-1] + "+00:00"
    try:
        parsed = datetime.fromisoformat(normalized)
    except ValueError:
        return None
    if parsed.tzinfo is None:
        parsed = parsed.replace(tzinfo=timezone.utc)
    return parsed.astimezone(timezone.utc)


def _cargo_published_at(name: str, version: str) -> datetime | None:
    payload = _http_json(f"https://crates.io/api/v1/crates/{name}/{version}")
    if not payload:
        return None
    version_info = payload.get("version") or {}
    return _parse_timestamp(version_info.get("created_at", ""))


def _pypi_published_at(name: str, version: str) -> datetime | None:
    payload = _http_json(f"https://pypi.org/pypi/{name}/{version}/json")
    if not payload:
        return None
    info = payload.get("info") or {}
    upload_time = info.get("upload_time") or info.get("upload_time_iso")
    if upload_time:
        return _parse_timestamp(upload_time)
    urls = payload.get("urls") or []
    for entry in urls:
        upload_time = entry.get("upload-time") or entry.get("upload_time")
        if upload_time:
            return _parse_timestamp(upload_time)
    return None


def _maven_published_at(coordinates: str, version: str) -> datetime | None:
    group, artifact = coordinates.split(":", 1)
    group_path = group.replace(".", "/")
    pom_url = (
        f"https://repo1.maven.org/maven2/{group_path}/{artifact}/{version}/"
        f"{artifact}-{version}.pom"
    )
    return _http_last_modified(pom_url)


def _bcr_published_at(module: str, version: str) -> datetime | None:
    metadata_url = (
        "https://raw.githubusercontent.com/bazelbuild/bazel-central-registry/main/"
        f"modules/{module}/metadata.json"
    )
    payload = _http_json(metadata_url)
    if payload:
        timestamps = payload.get("version_release_timestamps") or {}
        raw = timestamps.get(version)
        if raw:
            return _parse_timestamp(raw)

    commits_url = (
        "https://api.github.com/repos/bazelbuild/bazel-central-registry/commits"
        f"?path=modules/{module}/{version}/source.json&per_page=1"
    )
    commits = _http_json(commits_url)
    if not commits:
        return None
    commit_date = ((commits[0] or {}).get("commit") or {}).get("committer", {}).get("date")
    if not commit_date:
        return None
    return _parse_timestamp(commit_date)


def _nuget_published_at(package_id: str, version: str) -> datetime | None:
    registration_url = (
        "https://api.nuget.org/v3/registration5-semver1/"
        f"{package_id.lower()}/{version.lower()}.json"
    )
    payload = _http_json(registration_url)
    if not payload:
        return None
    published = payload.get("published")
    if not published:
        catalog_entry = payload.get("catalogEntry") or {}
        published = catalog_entry.get("published")
    if not published:
        return None
    return _parse_timestamp(published)


def _published_at(kind: str, name: str, version: str) -> datetime | None:
    if kind == "cargo":
        return _cargo_published_at(name, version)
    if kind == "pip":
        return _pypi_published_at(name, version)
    if kind == "maven":
        return _maven_published_at(name, version)
    if kind == "bcr":
        return _bcr_published_at(name, version)
    if kind == "nuget":
        return _nuget_published_at(name, version)
    return None


def _format_age(published_at: datetime, now: datetime) -> str:
    delta = now - published_at
    days = delta.total_seconds() / 86400.0
    return f"{days:.1f} days ago"


def check_release_ages(
    root: Path,
    base_ref: str | None,
    minimum_age: timedelta,
    now: datetime | None = None,
) -> list[str]:
    if now is None:
        now = datetime.now(timezone.utc)
    changed_paths = _changed_watched_files(root, base_ref)
    if not changed_paths:
        return []
    new_versions = _collect_new_versions(root, base_ref, changed_paths)
    if not new_versions:
        return []
    failures: list[str] = []
    skipped: list[str] = []
    cargo_lock_text = _read_file_at_ref(root, "HEAD", "Cargo.Bazel.lock")
    local_cargo = local_cargo_crates_from_bazel_lock(cargo_lock_text)
    for kind, name, version in sorted(new_versions):
        if kind == "cargo" and (name, version) in local_cargo:
            skipped.append(f"{kind} {name} {version}")
            continue
        published_at = _published_at(kind, name, version)
        label = f"{kind} {name} {version}"
        if published_at is None:
            failures.append(f"{label}: could not determine publish time")
            continue
        age = now - published_at
        if age < minimum_age:
            failures.append(
                f"{label}: published {_format_age(published_at, now)} "
                f"(minimum {minimum_age.days} days)"
            )
    if skipped:
        print(
            "skipped local or untimestamped dependencies: " + ", ".join(skipped),
            file=sys.stderr,
        )
    return failures


def main() -> int:
    root = _repo_root()
    minimum_age = _minimum_age()
    base_ref = _resolve_base_ref(root)
    if base_ref:
        print(f"checking new dependency versions since {base_ref}")
    failures = check_release_ages(root, base_ref, minimum_age)
    if not failures:
        print(f"dependency release age check passed (minimum {minimum_age.days} days)")
        return 0
    print("dependency release age check failed:", file=sys.stderr)
    for failure in failures:
        print(f"  - {failure}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
