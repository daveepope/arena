#!/usr/bin/env python3

from __future__ import annotations

import json
import os
import socket
import subprocess
import sys
import time
import urllib.error
import urllib.request
from concurrent.futures import ThreadPoolExecutor, as_completed

from container_defaults import load_container_defaults

DEFAULT_SEVERITIES = "CRITICAL,HIGH"
SEVERITIES_ENV_VAR = "ARENA_DEFAULT_CONTAINER_CVE_SEVERITIES"
SHOW_IDS_ENV_VAR = "ARENA_CONTAINER_CVE_SHOW_IDS"
DEFAULT_PLATFORM = "linux/amd64"
SERVER_READY_TIMEOUT_SECONDS = 60.0
RATE_LIMIT_MARKER = "TOOMANYREQUESTS"


class ScanError(RuntimeError):
    def __init__(self, message: str, *, rate_limited: bool = False):
        super().__init__(message)
        self.rate_limited = rate_limited


_TRIVY_APPARENT_REPOS = (
    "trivy_linux_x86_64",
    "trivy_macos_x86_64",
    "trivy_macos_arm64",
)


def _trivy_canonical_repos(r) -> list[str]:
    mapping_path = r.Rlocation("_repo_mapping")
    if not mapping_path or not os.path.isfile(mapping_path):
        return []
    canonical_repos = []
    with open(mapping_path, encoding="utf-8") as f:
        for line in f:
            parts = line.rstrip("\n").split(",")
            if len(parts) != 3:
                continue
            source_repo, apparent_name, canonical_name = parts
            if source_repo == "" and apparent_name in _TRIVY_APPARENT_REPOS:
                canonical_repos.append(canonical_name)
    return canonical_repos


def find_trivy_bin() -> str:
    path = os.environ.get("ARENA_TRIVY_BIN")
    if path and os.path.isfile(path):
        return path

    from bazel_tools.tools.python.runfiles import runfiles

    r = runfiles.Create()
    if r is not None:
        for canonical_repo in _trivy_canonical_repos(r):
            p = r.Rlocation(f"{canonical_repo}/trivy")
            if p and os.path.isfile(p):
                return p

    raise RuntimeError(
        "trivy binary not found; set ARENA_TRIVY_BIN or run via "
        "`bazel run //scripts:check_container_cves`"
    )


def _free_port() -> int:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]


def start_trivy_server(trivy_bin: str) -> tuple[subprocess.Popen, str]:
    port = _free_port()
    server_url = f"http://127.0.0.1:{port}"
    process = subprocess.Popen(
        [trivy_bin, "server", "--listen", f"127.0.0.1:{port}"],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.PIPE,
        text=True,
    )
    try:
        _wait_for_server_ready(process, server_url)
    except Exception:
        stop_trivy_server(process)
        raise
    return process, server_url


def _wait_for_server_ready(process: subprocess.Popen, server_url: str) -> None:
    deadline = time.monotonic() + SERVER_READY_TIMEOUT_SECONDS
    while time.monotonic() < deadline:
        if process.poll() is not None:
            stderr = process.stderr.read() if process.stderr else ""
            raise RuntimeError(f"trivy server exited early (code {process.returncode}): {stderr}")
        try:
            with urllib.request.urlopen(f"{server_url}/healthz", timeout=2) as resp:
                if resp.status == 200:
                    return
        except (urllib.error.URLError, TimeoutError):
            pass
        time.sleep(0.5)
    raise RuntimeError(f"trivy server did not become ready within {SERVER_READY_TIMEOUT_SECONDS}s")


def stop_trivy_server(process: subprocess.Popen) -> None:
    process.terminate()
    try:
        process.wait(timeout=10)
    except subprocess.TimeoutExpired:
        process.kill()
        process.wait()


def severities_from_env() -> str:
    return os.environ.get(SEVERITIES_ENV_VAR, DEFAULT_SEVERITIES).strip() or DEFAULT_SEVERITIES


def show_vulnerability_ids_from_env() -> bool:
    return os.environ.get(SHOW_IDS_ENV_VAR, "").strip().lower() in ("1", "true", "yes")


def scan_image(trivy_bin: str, server_url: str, image_ref: str, severities: str) -> dict:
    result = subprocess.run(
        [
            trivy_bin,
            "image",
            "--server",
            server_url,
            "--platform",
            DEFAULT_PLATFORM,
            "--format",
            "json",
            "--severity",
            severities,
            "--quiet",
            image_ref,
        ],
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        stderr = result.stderr.strip()
        rate_limited = RATE_LIMIT_MARKER in stderr
        raise ScanError(f"trivy scan failed for {image_ref}: {stderr}", rate_limited=rate_limited)
    return json.loads(result.stdout)


def scan_all_images(
    trivy_bin: str,
    server_url: str,
    entries: list[dict[str, str]],
    severities: str,
) -> dict[str, dict | ScanError]:
    scan_results: dict[str, dict | ScanError] = {}
    with ThreadPoolExecutor(max_workers=max(len(entries), 1)) as pool:
        future_to_id = {
            pool.submit(
                scan_image, trivy_bin, server_url, f"{entry['image']}:{entry['tag']}", severities
            ): entry["id"]
            for entry in entries
        }
        for future in as_completed(future_to_id):
            image_id = future_to_id[future]
            try:
                scan_results[image_id] = future.result()
            except ScanError as exc:
                scan_results[image_id] = exc
    return scan_results


class Vulnerability:
    def __init__(self, severity: str):
        self.severity = severity
        self.packages: list[str] = []

    def add_package(self, package: str | None) -> None:
        if package and package not in self.packages:
            self.packages.append(package)


def distinct_vulnerabilities(raw: dict) -> dict[str, Vulnerability]:
    by_id: dict[str, Vulnerability] = {}
    for result in raw.get("Results") or []:
        for vuln in result.get("Vulnerabilities") or []:
            vuln_id = vuln.get("VulnerabilityID")
            if not vuln_id:
                continue
            entry = by_id.setdefault(vuln_id, Vulnerability(vuln.get("Severity", "UNKNOWN")))
            entry.add_package(vuln.get("PkgName"))
    return by_id


def severity_counts(raw: dict) -> dict[str, int]:
    counts: dict[str, int] = {}
    for vuln in distinct_vulnerabilities(raw).values():
        counts[vuln.severity] = counts.get(vuln.severity, 0) + 1
    return counts


_TABLE_COLUMNS = ("ID", "IMAGE", "DISTINCT CRITICAL CVEs", "DISTINCT HIGH CVEs")


def build_row(entry: dict[str, str], raw: dict) -> dict[str, str]:
    counts = severity_counts(raw)
    return {
        "ID": entry["id"],
        "IMAGE": f"{entry['image']}:{entry['tag']}",
        "DISTINCT CRITICAL CVEs": str(counts.get("CRITICAL", 0)),
        "DISTINCT HIGH CVEs": str(counts.get("HIGH", 0)),
    }


def render_table(rows: list[dict[str, str]]) -> str:
    widths = {
        column: max(len(column), *(len(row[column]) for row in rows)) if rows else len(column)
        for column in _TABLE_COLUMNS
    }

    def border(left: str, fill: str, sep: str, right: str) -> str:
        return left + sep.join(fill * (widths[column] + 2) for column in _TABLE_COLUMNS) + right

    def format_row(values: dict[str, str]) -> str:
        cells = (f" {values[column].ljust(widths[column])} " for column in _TABLE_COLUMNS)
        return "│" + "│".join(cells) + "│"

    lines = [
        border("┌", "─", "┬", "┐"),
        format_row({column: column for column in _TABLE_COLUMNS}),
        border("├", "─", "┼", "┤"),
    ]
    lines.extend(format_row(row) for row in rows)
    lines.append(border("└", "─", "┴", "┘"))
    return "\n".join(lines)


def scan_error_entries(
    entries: list[dict[str, str]],
    scan_results: dict[str, dict | ScanError],
) -> list[tuple[dict[str, str], ScanError]]:
    return [
        (entry, scan_results[entry["id"]])
        for entry in entries
        if isinstance(scan_results[entry["id"]], ScanError)
    ]


def scan_error_reason(error: ScanError) -> str:
    if error.rate_limited:
        return (
            "rate limited by the registry (usually a rolling per-IP limit, e.g. "
            "Docker Hub's anonymous pull limit, that clears on its own)"
        )
    return str(error)


def main() -> int:
    if hasattr(sys.stdout, "reconfigure"):
        sys.stdout.reconfigure(encoding="utf-8")
    entries = load_container_defaults()
    trivy_bin = find_trivy_bin()
    severities = severities_from_env()
    show_ids = show_vulnerability_ids_from_env()
    process, server_url = start_trivy_server(trivy_bin)
    try:
        scan_results = scan_all_images(trivy_bin, server_url, entries, severities)

        errors = scan_error_entries(entries, scan_results)
        if errors:
            print("container image CVE check aborted: could not scan the following image(s):")
            for entry, error in errors:
                print(f"  {entry['id']}: {scan_error_reason(error)}")
            return 1

        rows = [build_row(entry, scan_results[entry["id"]]) for entry in entries]
        print("Best effort CVE search, informational only. Zero CVEs is not guaranteed.")
        print(render_table(rows))

        if show_ids:
            for entry in entries:
                vulns = distinct_vulnerabilities(scan_results[entry["id"]])
                if vulns:
                    details = ", ".join(
                        f"{vuln_id} ({'/'.join(vuln.packages)})" if vuln.packages else vuln_id
                        for vuln_id, vuln in vulns.items()
                    )
                    print(f"{entry['id']}: {details}")

        return 0
    finally:
        stop_trivy_server(process)


if __name__ == "__main__":
    sys.exit(main())
