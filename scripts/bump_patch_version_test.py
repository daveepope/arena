from __future__ import annotations

import subprocess
import tempfile
import unittest
from pathlib import Path

from bump_patch_version import bump_patch_release_from_base
from arena_version import is_synced, read_version, write_version


class BumpPatchReleaseFromBaseTest(unittest.TestCase):
    def _write_workspace_files(self, root: Path, version: str) -> None:
        write_version(root, version)
        (root / "Cargo.toml").write_text(
            f'[workspace.package]\nversion = "{version}"\n',
            encoding="utf-8",
        )
        (root / "MODULE.bazel").write_text(
            f'module(\n    name = "arena",\n    version = "{version}",\n)\n',
            encoding="utf-8",
        )
        (root / "arena-pytest").mkdir(exist_ok=True)
        (root / "arena-pytest/pyproject.toml").write_text(
            '[project]\ndynamic = ["version"]\n',
            encoding="utf-8",
        )

    def _setup_git_repo(self, root: Path, base_version: str, head_version: str) -> None:
        self._write_workspace_files(root, base_version)
        subprocess.run(["git", "init"], cwd=root, check=True, capture_output=True)
        subprocess.run(["git", "config", "user.email", "test@example.com"], cwd=root, check=True)
        subprocess.run(["git", "config", "user.name", "test"], cwd=root, check=True)
        subprocess.run(["git", "add", "."], cwd=root, check=True)
        subprocess.run(["git", "commit", "-m", "base"], cwd=root, check=True)
        subprocess.run(["git", "branch", "base"], cwd=root, check=True)
        if base_version != head_version:
            self._write_workspace_files(root, head_version)
            subprocess.run(["git", "add", "."], cwd=root, check=True)
            subprocess.run(["git", "commit", "-m", "head"], cwd=root, check=True)

    def test_bump_patch_release_from_base_stale_head_bumps_patch(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self._setup_git_repo(root, "1.1.0", "1.1.0")
            head, changed = bump_patch_release_from_base(root, "base")
            self.assertEqual(head, "1.1.1")
            self.assertEqual(read_version(root), "1.1.1")
            self.assertTrue(is_synced(root))
            self.assertIn("Cargo.toml", changed)
            self.assertIn("MODULE.bazel", changed)

    def test_bump_patch_release_from_base_unsynced_derived_files_syncs(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self._setup_git_repo(root, "1.1.0", "1.1.0")
            write_version(root, "1.1.1")
            (root / "Cargo.toml").write_text(
                '[workspace.package]\nversion = "1.1.0"\n',
                encoding="utf-8",
            )
            head, changed = bump_patch_release_from_base(root, "base")
            self.assertEqual(head, "1.1.1")
            self.assertIn("Cargo.toml", changed)
            self.assertTrue(is_synced(root))

    def test_bump_patch_release_from_base_head_above_base_keeps_head(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self._setup_git_repo(root, "1.1.0", "1.1.2")
            head, changed = bump_patch_release_from_base(root, "base")
            self.assertEqual(head, "1.1.2")
            self.assertEqual(read_version(root), "1.1.2")
            self.assertEqual(changed, [])

    def test_bump_patch_release_from_base_master_advanced_rebumps(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self._setup_git_repo(root, "1.1.0", "1.1.1")
            subprocess.run(["git", "branch", "merged", "HEAD"], cwd=root, check=True)
            head, _changed = bump_patch_release_from_base(root, "merged")
            self.assertEqual(head, "1.1.2")
            self.assertEqual(read_version(root), "1.1.2")


if __name__ == "__main__":
    unittest.main()
