from __future__ import annotations

import json
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from arena_version import (
    bump_release_version,
    is_synced,
    prepare_release_version,
    read_version,
    read_version_from_git_ref,
    release_version_only,
    resolve_semver_level_from_event,
    sync_workspace_version,
    write_version,
)


class BumpReleaseVersionTest(unittest.TestCase):
    def test_bump_release_version_patch_increments_patch(self) -> None:
        self.assertEqual(bump_release_version("1.0.0", "patch"), "1.0.1")

    def test_bump_release_version_minor_increments_minor(self) -> None:
        self.assertEqual(bump_release_version("1.0.0", "minor"), "1.1.0")

    def test_bump_release_version_major_increments_major(self) -> None:
        self.assertEqual(bump_release_version("1.0.0", "major"), "2.0.0")


class ReleaseVersionOnlyTest(unittest.TestCase):
    def test_release_version_only_dev_suffix_returns_base(self) -> None:
        self.assertEqual(release_version_only("1.0.1.dev12345"), "1.0.1")


class ResolveSemverLevelTest(unittest.TestCase):
    def test_resolve_semver_level_major_label_returns_major(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            event = Path(tmp) / "event.json"
            event.write_text(
                json.dumps(
                    {
                        "pull_request": {
                            "labels": [{"name": "semver:minor"}, {"name": "semver:major"}],
                        }
                    }
                ),
                encoding="utf-8",
            )
            self.assertEqual(resolve_semver_level_from_event(event), "major")

    def test_resolve_semver_level_no_label_returns_patch(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            event = Path(tmp) / "event.json"
            event.write_text(json.dumps({"pull_request": {"labels": []}}), encoding="utf-8")
            self.assertEqual(resolve_semver_level_from_event(event), "patch")


class ReadVersionFromGitRefTest(unittest.TestCase):
    def test_read_version_from_git_ref_missing_version_uses_cargo(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            subprocess.run(["git", "init"], cwd=root, check=True, capture_output=True)
            subprocess.run(["git", "config", "user.email", "test@example.com"], cwd=root, check=True)
            subprocess.run(["git", "config", "user.name", "test"], cwd=root, check=True)
            (root / "Cargo.toml").write_text(
                '[workspace.package]\nversion = "0.4.0-beta.1"\n',
                encoding="utf-8",
            )
            subprocess.run(["git", "add", "Cargo.toml"], cwd=root, check=True)
            subprocess.run(["git", "commit", "-m", "init"], cwd=root, check=True)
            self.assertEqual(read_version_from_git_ref(root, "HEAD"), "0.4.0")


class PrepareReleaseVersionTest(unittest.TestCase):
    def test_prepare_release_version_new_epoch_bumps_current(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            write_version(root, "1.0.0")
            (root / "Cargo.toml").write_text(
                '[workspace.package]\nversion = "1.0.0"\n',
                encoding="utf-8",
            )
            (root / "MODULE.bazel").write_text(
                'module(\n    name = "arena",\n    version = "1.0.0",\n)\n',
                encoding="utf-8",
            )
            (root / "arena-pytest").mkdir()
            (root / "arena-pytest/pyproject.toml").write_text(
                '[project]\ndynamic = ["version"]\n',
                encoding="utf-8",
            )

            with patch(
                "arena_version.read_release_version_from_git_ref",
                return_value=("0.4.0", False),
            ):
                target, _changed = prepare_release_version(root, "origin/master", "patch")

            self.assertEqual(target, "1.0.1")
            self.assertEqual(read_version(root), "1.0.1")

    def test_prepare_release_version_higher_pr_keeps_version(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            write_version(root, "1.1.0")
            (root / "Cargo.toml").write_text(
                '[workspace.package]\nversion = "1.1.0"\n',
                encoding="utf-8",
            )
            (root / "MODULE.bazel").write_text(
                'module(\n    name = "arena",\n    version = "1.1.0",\n)\n',
                encoding="utf-8",
            )
            (root / "arena-pytest").mkdir()
            (root / "arena-pytest/pyproject.toml").write_text(
                '[project]\ndynamic = ["version"]\n',
                encoding="utf-8",
            )

            with patch(
                "arena_version.read_release_version_from_git_ref",
                return_value=("1.0.0", True),
            ):
                target, changed = prepare_release_version(root, "origin/master", "patch")

            self.assertEqual(target, "1.1.0")
            self.assertEqual(changed, [])
            self.assertEqual(read_version(root), "1.1.0")


class SyncWorkspaceVersionTest(unittest.TestCase):
    def test_sync_workspace_version_updates_cargo_and_module(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            write_version(root, "2.3.4")
            (root / "Cargo.toml").write_text(
                '[workspace.package]\nversion = "1.0.0"\n',
                encoding="utf-8",
            )
            (root / "MODULE.bazel").write_text(
                'module(\n    name = "arena",\n    version = "1.0.0",\n)\n',
                encoding="utf-8",
            )
            (root / "arena-pytest").mkdir()
            (root / "arena-pytest/pyproject.toml").write_text(
                '[project]\ndynamic = ["version"]\n',
                encoding="utf-8",
            )
            changed = sync_workspace_version(root)
            self.assertIn("Cargo.toml", changed)
            self.assertIn("MODULE.bazel", changed)
            self.assertTrue(is_synced(root))


if __name__ == "__main__":
    unittest.main()
