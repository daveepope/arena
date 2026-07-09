from __future__ import annotations

import subprocess
import tempfile
import unittest
from pathlib import Path

from arena_version import (
    is_synced,
    read_version_from_git_ref,
    release_lockfiles_need_repin,
    release_version_increased,
    release_version_only,
    sync_workspace_version,
    write_version,
)


class ReleaseVersionOnlyTest(unittest.TestCase):
    def test_release_version_only_dev_suffix_returns_base(self) -> None:
        self.assertEqual(release_version_only("1.0.1.dev12345"), "1.0.1")


class ReleaseVersionIncreasedTest(unittest.TestCase):
    def test_release_version_increased_patch_returns_true(self) -> None:
        self.assertTrue(release_version_increased("1.0.0", "1.0.1"))

    def test_release_version_increased_same_returns_false(self) -> None:
        self.assertFalse(release_version_increased("1.0.0", "1.0.0"))

    def test_release_version_increased_downgrade_returns_false(self) -> None:
        self.assertFalse(release_version_increased("1.1.0", "1.0.0"))


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
