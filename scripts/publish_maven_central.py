#!/usr/bin/env python3

from __future__ import annotations

import argparse
import base64
import hashlib
import json
import os
import subprocess
import sys
import time
import urllib.error
import urllib.request
import uuid
import xml.etree.ElementTree
import zipfile
from pathlib import Path

PUBLISHER_UPLOAD_URL = "https://central.sonatype.com/api/v1/publisher/upload"
PUBLISHER_STATUS_URL = "https://central.sonatype.com/api/v1/publisher/status"
HTTP_TIMEOUT_SECONDS = 60
DEPLOYMENT_POLL_INTERVAL_SECONDS = 10
DEPLOYMENT_POLL_TIMEOUT_SECONDS = 600
TERMINAL_FAILURE_STATES = {"FAILED"}
TERMINAL_SUCCESS_STATES = {"PUBLISHED", "PUBLISHING"}
POM_NAMESPACE = {"m": "http://maven.apache.org/POM/4.0.0"}


def _pom_coordinates(pom_path: Path) -> tuple[str, str, str]:
    root = xml.etree.ElementTree.parse(pom_path).getroot()
    group_id = root.findtext("m:groupId", namespaces=POM_NAMESPACE)
    artifact_id = root.findtext("m:artifactId", namespaces=POM_NAMESPACE)
    version = root.findtext("m:version", namespaces=POM_NAMESPACE)
    if not group_id or not artifact_id or not version:
        raise ValueError(f"pom at {pom_path} is missing groupId/artifactId/version")
    return group_id, artifact_id, version


def _write_checksums(path: Path) -> None:
    contents = path.read_bytes()
    path.with_name(path.name + ".md5").write_text(hashlib.md5(contents).hexdigest())
    path.with_name(path.name + ".sha1").write_text(hashlib.sha1(contents).hexdigest())


def _sign(path: Path, passphrase: str) -> Path:
    signature_path = path.with_name(path.name + ".asc")
    subprocess.run(
        [
            "gpg",
            "--batch",
            "--yes",
            "--pinentry-mode",
            "loopback",
            "--passphrase-fd",
            "0",
            "--detach-sign",
            "--armor",
            "--output",
            str(signature_path),
            str(path),
        ],
        input=passphrase,
        text=True,
        check=True,
    )
    return signature_path


def _build_bundle(
    staging_dir: Path,
    group_id: str,
    artifact_id: str,
    version: str,
    jar_path: Path,
    pom_path: Path,
    sources_jar_path: Path,
    javadoc_jar_path: Path,
    classifier_jars: dict[str, Path],
    passphrase: str,
) -> Path:
    layout_dir = staging_dir / group_id.replace(".", "/") / artifact_id / version
    layout_dir.mkdir(parents=True, exist_ok=True)

    artifact_prefix = f"{artifact_id}-{version}"
    files_to_upload = {
        f"{artifact_prefix}.jar": jar_path,
        f"{artifact_prefix}.pom": pom_path,
        f"{artifact_prefix}-sources.jar": sources_jar_path,
        f"{artifact_prefix}-javadoc.jar": javadoc_jar_path,
    }
    for classifier, classifier_jar_path in classifier_jars.items():
        files_to_upload[f"{artifact_prefix}-{classifier}.jar"] = classifier_jar_path

    staged_paths: list[Path] = []
    for target_name, source_path in files_to_upload.items():
        staged_path = layout_dir / target_name
        staged_path.write_bytes(source_path.read_bytes())
        staged_paths.append(staged_path)

    for staged_path in staged_paths:
        _write_checksums(staged_path)
        _sign(staged_path, passphrase)

    bundle_path = staging_dir / f"{artifact_prefix}-bundle.zip"
    with zipfile.ZipFile(bundle_path, "w", zipfile.ZIP_DEFLATED) as bundle:
        for file_path in sorted(layout_dir.rglob("*")):
            if file_path.is_file():
                bundle.write(file_path, file_path.relative_to(staging_dir))

    return bundle_path


def _authorization_header(username: str, password: str) -> str:
    token = base64.b64encode(f"{username}:{password}".encode()).decode()
    return f"Bearer {token}"


def _upload(bundle_path: Path, username: str, password: str) -> str:
    boundary = uuid.uuid4().hex
    body = bytearray()
    body += f"--{boundary}\r\n".encode()
    body += (
        f'Content-Disposition: form-data; name="bundle"; filename="{bundle_path.name}"\r\n'
    ).encode()
    body += b"Content-Type: application/zip\r\n\r\n"
    body += bundle_path.read_bytes()
    body += f"\r\n--{boundary}--\r\n".encode()

    request = urllib.request.Request(
        f"{PUBLISHER_UPLOAD_URL}?publishingType=AUTOMATIC",
        data=bytes(body),
        method="POST",
        headers={
            "Authorization": _authorization_header(username, password),
            "Content-Type": f"multipart/form-data; boundary={boundary}",
        },
    )
    try:
        with urllib.request.urlopen(request, timeout=HTTP_TIMEOUT_SECONDS) as response:
            deployment_id = response.read().decode().strip()
            print(f"Central Portal accepted upload, deployment id: {deployment_id}")
            return deployment_id
    except urllib.error.HTTPError as error:
        print(f"Central Portal upload failed ({error.code}): {error.read().decode()}", file=sys.stderr)
        raise


def _deployment_state(deployment_id: str, username: str, password: str) -> str:
    request = urllib.request.Request(
        f"{PUBLISHER_STATUS_URL}?id={deployment_id}",
        method="POST",
        headers={"Authorization": _authorization_header(username, password)},
    )
    with urllib.request.urlopen(request, timeout=HTTP_TIMEOUT_SECONDS) as response:
        payload = json.loads(response.read().decode())
    return payload["deploymentState"]


def _await_publish(deployment_id: str, username: str, password: str) -> None:
    deadline = time.monotonic() + DEPLOYMENT_POLL_TIMEOUT_SECONDS
    while time.monotonic() < deadline:
        state = _deployment_state(deployment_id, username, password)
        print(f"Central Portal deployment {deployment_id} state: {state}")
        if state in TERMINAL_FAILURE_STATES:
            raise RuntimeError(f"Central Portal rejected deployment {deployment_id} (state={state})")
        if state in TERMINAL_SUCCESS_STATES:
            return
        time.sleep(DEPLOYMENT_POLL_INTERVAL_SECONDS)
    raise TimeoutError(
        f"Central Portal deployment {deployment_id} did not reach a terminal state "
        f"within {DEPLOYMENT_POLL_TIMEOUT_SECONDS}s"
    )


def _parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("jar_path")
    parser.add_argument("pom_path")
    parser.add_argument("sources_jar_path")
    parser.add_argument("javadoc_jar_path")
    parser.add_argument(
        "--classifier-jar",
        action="append",
        default=[],
        metavar="CLASSIFIER=PATH",
        help="Additional classifier=jar_path pair to publish alongside the main artifact",
    )
    return parser.parse_args(argv)


def _parse_classifier_jars(entries: list[str]) -> dict[str, Path]:
    classifier_jars: dict[str, Path] = {}
    for entry in entries:
        classifier, separator, path = entry.partition("=")
        if not separator or not classifier or not path:
            raise ValueError(f"invalid --classifier-jar value: {entry!r}, expected CLASSIFIER=PATH")
        classifier_jars[classifier] = Path(path)
    return classifier_jars


def main() -> None:
    args = _parse_args(sys.argv[1:])
    classifier_jars = _parse_classifier_jars(args.classifier_jar)

    passphrase = os.environ["GPG_PASSPHRASE"]
    username = os.environ["CENTRAL_TOKEN_USERNAME"]
    password = os.environ["CENTRAL_TOKEN_PASSWORD"]

    group_id, artifact_id, version = _pom_coordinates(Path(args.pom_path))

    staging_dir = Path(os.environ.get("RUNNER_TEMP", "/tmp")) / f"maven-publish-{uuid.uuid4().hex}"
    staging_dir.mkdir(parents=True, exist_ok=True)

    bundle_path = _build_bundle(
        staging_dir=staging_dir,
        group_id=group_id,
        artifact_id=artifact_id,
        version=version,
        jar_path=Path(args.jar_path),
        pom_path=Path(args.pom_path),
        sources_jar_path=Path(args.sources_jar_path),
        javadoc_jar_path=Path(args.javadoc_jar_path),
        classifier_jars=classifier_jars,
        passphrase=passphrase,
    )
    deployment_id = _upload(bundle_path, username, password)
    _await_publish(deployment_id, username, password)


if __name__ == "__main__":
    main()
