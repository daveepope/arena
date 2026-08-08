from __future__ import annotations

import os
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from arena_version import (
    bump_major_version,
    bump_minor_version,
    bump_patch_version,
    bump_version,
    higher_release_version,
    is_synced,
    read_version,
    read_version_from_git_ref,
    release_lockfiles_need_repin,
    release_version_increased,
    release_version_only,
    repin_release_lockfiles,
    sync_workspace_version,
    workspace_version_in_cargo_bazel_lock,
    write_version,
)


class ReleaseVersionOnlyTest(unittest.TestCase):
    def test_release_version_only_dev_suffix_returns_base(self) -> None:
        self.assertEqual(release_version_only("1.0.1.dev12345"), "1.0.1")

    def test_release_version_only_invalid_raises(self) -> None:
        with self.assertRaises(ValueError):
            release_version_only("not-a-version")


class ReadVersionTest(unittest.TestCase):
    def test_read_version_empty_file_raises(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "VERSION").write_text("\n", encoding="utf-8")
            with self.assertRaises(ValueError):
                read_version(root)

    def test_write_version_roundtrip_returns_value(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            write_version(root, "3.4.5")
            self.assertEqual(read_version(root), "3.4.5")


class BumpPatchVersionTest(unittest.TestCase):
    def test_bump_patch_version_increments_patch(self) -> None:
        self.assertEqual(bump_patch_version("1.1.0"), "1.1.1")


class BumpMinorVersionTest(unittest.TestCase):
    def test_bump_minor_version_increments_minor_resets_patch(self) -> None:
        self.assertEqual(bump_minor_version("1.1.5"), "1.2.0")


class BumpMajorVersionTest(unittest.TestCase):
    def test_bump_major_version_increments_major_resets_minor_patch(self) -> None:
        self.assertEqual(bump_major_version("1.5.3"), "2.0.0")


class BumpVersionTest(unittest.TestCase):
    def test_bump_version_kind_patch_delegates_to_bump_patch_version(self) -> None:
        self.assertEqual(bump_version("1.1.0", "patch"), "1.1.1")

    def test_bump_version_kind_minor_delegates_to_bump_minor_version(self) -> None:
        self.assertEqual(bump_version("1.1.0", "minor"), "1.2.0")

    def test_bump_version_kind_major_delegates_to_bump_major_version(self) -> None:
        self.assertEqual(bump_version("1.1.0", "major"), "2.0.0")

    def test_bump_version_unknown_kind_raises_value_error(self) -> None:
        with self.assertRaises(ValueError):
            bump_version("1.1.0", "typo")


class HigherReleaseVersionTest(unittest.TestCase):
    def test_higher_release_version_a_greater_returns_a(self) -> None:
        self.assertEqual(higher_release_version("1.2.0", "1.1.0"), "1.2.0")

    def test_higher_release_version_b_greater_returns_b(self) -> None:
        self.assertEqual(higher_release_version("1.1.0", "1.2.0"), "1.2.0")

    def test_higher_release_version_equal_returns_a(self) -> None:
        self.assertEqual(higher_release_version("1.1.0", "1.1.0"), "1.1.0")


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

    def test_read_version_from_git_ref_module_bazel_fallback(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            subprocess.run(["git", "init"], cwd=root, check=True, capture_output=True)
            subprocess.run(["git", "config", "user.email", "test@example.com"], cwd=root, check=True)
            subprocess.run(["git", "config", "user.name", "test"], cwd=root, check=True)
            (root / "MODULE.bazel").write_text(
                'module(\n    name = "arena",\n    version = "2.1.0",\n)\n',
                encoding="utf-8",
            )
            subprocess.run(["git", "add", "MODULE.bazel"], cwd=root, check=True)
            subprocess.run(["git", "commit", "-m", "init"], cwd=root, check=True)
            self.assertEqual(read_version_from_git_ref(root, "HEAD"), "2.1.0")

    def test_read_version_from_git_ref_missing_ref_raises(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            subprocess.run(["git", "init"], cwd=root, check=True, capture_output=True)
            with self.assertRaises(ValueError):
                read_version_from_git_ref(root, "HEAD")


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

    def test_release_lockfiles_need_repin_aligned_returns_false(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            lock = """
{
  "crates": {
    "arena 1.0.1": {
      "name": "arena",
      "version": "1.0.1",
      "package_url": null
    }
  }
}
"""
            (root / "Cargo.Bazel.lock").write_text(lock, encoding="utf-8")
            self.assertFalse(release_lockfiles_need_repin(root, "1.0.1"))

    def test_workspace_version_in_cargo_bazel_lock_missing_returns_none(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self.assertIsNone(workspace_version_in_cargo_bazel_lock(root))

    def test_workspace_version_in_cargo_bazel_lock_reads_arena_version(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            lock = """
{
  "crates": {
    "arena 9.8.7": {
      "name": "arena",
      "version": "9.8.7",
      "package_url": null
    }
  }
}
"""
            (root / "Cargo.Bazel.lock").write_text(lock, encoding="utf-8")
            self.assertEqual(workspace_version_in_cargo_bazel_lock(root), "9.8.7")

    def test_repin_release_lockfiles_invokes_bazel(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            with patch("arena_version.subprocess.run") as run:
                repin_release_lockfiles(root)
                self.assertEqual(run.call_count, 2)

    def test_repin_release_lockfiles_passes_bazel_config(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            with patch.dict(os.environ, {"ARENA_BAZEL_CONFIG": "ci"}):
                with patch("arena_version.subprocess.run") as run:
                    repin_release_lockfiles(root)
                    build_args = run.call_args_list[0].args[0]
                    self.assertIn("--config=ci", build_args)


if __name__ == "__main__":
    unittest.main()
