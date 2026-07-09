from __future__ import annotations

import subprocess
import tempfile
import unittest
from pathlib import Path

from check_version_bump import validate_version_bump
from check_release_lockfiles import release_lockfiles_match_version
from arena_version import write_version


class ValidateVersionBumpTest(unittest.TestCase):
    def _setup_git_repo(self, root: Path, version: str) -> None:
        write_version(root, version)
        subprocess.run(["git", "init"], cwd=root, check=True, capture_output=True)
        subprocess.run(["git", "config", "user.email", "test@example.com"], cwd=root, check=True)
        subprocess.run(["git", "config", "user.name", "test"], cwd=root, check=True)
        subprocess.run(["git", "add", "VERSION"], cwd=root, check=True)
        subprocess.run(["git", "commit", "-m", "init"], cwd=root, check=True)

    def test_validate_version_bump_increased_version_returns_zero(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self._setup_git_repo(root, "1.0.0")
            subprocess.run(["git", "branch", "base"], cwd=root, check=True)
            write_version(root, "1.0.1")
            code, message = validate_version_bump(root, "base")
            self.assertEqual(code, 0)
            self.assertIn("1.0.0 -> 1.0.1", message)

    def test_validate_version_bump_unchanged_version_returns_one(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self._setup_git_repo(root, "1.0.0")
            code, message = validate_version_bump(root, "HEAD")
            self.assertEqual(code, 1)
            self.assertIn("still 1.0.0", message)

    def test_validate_version_bump_downgrade_returns_one(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self._setup_git_repo(root, "1.1.0")
            subprocess.run(["git", "branch", "base"], cwd=root, check=True)
            write_version(root, "1.0.0")
            code, message = validate_version_bump(root, "base")
            self.assertEqual(code, 1)
            self.assertIn("must increase", message)

    def test_validate_version_bump_missing_base_ref_returns_one(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            write_version(root, "1.0.0")
            code, message = validate_version_bump(root, "missing-ref")
            self.assertEqual(code, 1)
            self.assertIn("could not read base version", message)


class ReleaseLockfilesMatchVersionTest(unittest.TestCase):
    def test_release_lockfiles_match_version_mismatch_returns_false(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            write_version(root, "1.0.1")
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
            ok, message = release_lockfiles_match_version(root)
            self.assertFalse(ok)
            self.assertIn("1.0.1", message)

    def test_release_lockfiles_match_version_aligned_returns_true(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            write_version(root, "1.0.1")
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
            ok, message = release_lockfiles_match_version(root)
            self.assertTrue(ok)
            self.assertIn("1.0.1", message)


if __name__ == "__main__":
    unittest.main()
