from __future__ import annotations

import json
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from arena_version import (
    bump_release_version,
    cargo_workspace_version,
    is_synced,
    prepare_preview_from_base,
    prepare_release_from_base,
    read_version,
    read_version_from_git_ref,
    release_lockfiles_need_repin,
    release_target_from_base,
    release_version_only,
    resolve_semver_level_from_event,
    resolve_semver_level_from_labels,
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
        self.assertEqual(
            resolve_semver_level_from_labels(["semver:minor", "semver:major"]),
            "major",
        )

    def test_resolve_semver_level_minor_label_returns_minor(self) -> None:
        self.assertEqual(resolve_semver_level_from_labels(["semver:minor"]), "minor")

    def test_resolve_semver_level_no_label_returns_patch(self) -> None:
        self.assertEqual(resolve_semver_level_from_labels([]), "patch")

    def test_resolve_semver_level_from_event_no_label_returns_patch(self) -> None:
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


class PrepareReleaseFromBaseTest(unittest.TestCase):
    def _setup_root(self, root: Path, version: str) -> None:
        write_version(root, version)
        (root / "Cargo.toml").write_text(
            f'[workspace.package]\nversion = "{version}"\n',
            encoding="utf-8",
        )
        (root / "MODULE.bazel").write_text(
            f'module(\n    name = "arena",\n    version = "{version}",\n)\n',
            encoding="utf-8",
        )
        (root / "arena-pytest").mkdir()
        (root / "arena-pytest/pyproject.toml").write_text(
            '[project]\ndynamic = ["version"]\n',
            encoding="utf-8",
        )

    def test_prepare_preview_from_base_no_version_on_master_returns_first_release(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self._setup_root(root, "1.0.0")
            with patch(
                "arena_version.read_release_version_from_git_ref",
                return_value=("0.4.0", False),
            ):
                target, changed = prepare_preview_from_base(root, "origin/master", "minor")
            self.assertEqual(target, "1.0.0")
            self.assertEqual(changed, [])

    def test_release_target_from_base_version_on_master_uses_label(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self._setup_root(root, "1.0.0")
            with patch(
                "arena_version.read_release_version_from_git_ref",
                return_value=("1.0.0", True),
            ):
                self.assertEqual(release_target_from_base(root, "origin/master", "minor"), "1.1.0")

    def test_prepare_release_from_base_uses_base_not_branch(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self._setup_root(root, "1.3.0")
            with patch(
                "arena_version.read_release_version_from_git_ref",
                return_value=("1.0.0", True),
            ):
                target, changed = prepare_release_from_base(root, "origin/master", "minor")
            self.assertEqual(target, "1.1.0")
            self.assertEqual(read_version(root), "1.1.0")
            self.assertIn("VERSION", changed)

    def test_release_target_from_base_patch_returns_bumped(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self._setup_root(root, "1.0.0")
            subprocess.run(["git", "init"], cwd=root, check=True, capture_output=True)
            subprocess.run(["git", "config", "user.email", "test@example.com"], cwd=root, check=True)
            subprocess.run(["git", "config", "user.name", "test"], cwd=root, check=True)
            subprocess.run(["git", "add", "."], cwd=root, check=True)
            subprocess.run(["git", "commit", "-m", "init"], cwd=root, check=True)
            self.assertEqual(release_target_from_base(root, "HEAD", "patch"), "1.0.1")


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

    def test_release_lockfiles_need_repin_mismatch_returns_true(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            lock = """
{
  "crates": {
    "arena 1.0.0": {
      "name": "arena",
      "version": "1.0.0",
      "package_url": null
    }
  }
}
"""
            (root / "Cargo.Bazel.lock").write_text(lock, encoding="utf-8")
            self.assertTrue(release_lockfiles_need_repin(root, "1.0.1"))


if __name__ == "__main__":
    unittest.main()
