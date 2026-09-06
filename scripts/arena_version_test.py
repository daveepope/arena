from __future__ import annotations

import os
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from arena_version import (
    CARGO_VET_AUDITED_PACKAGE_COUNT_WATERMARK,
    CARGO_VET_EXEMPTED_PACKAGE_COUNT_WATERMARK,
    audit_arena_ffi_binary,
    bump_major_version,
    bump_minor_version,
    bump_patch_version,
    bump_version,
    check_cargo_vet_watermarks,
    higher_release_version,
    is_synced,
    read_version,
    read_version_from_git_ref,
    release_lockfiles_need_repin,
    release_version_increased,
    release_version_only,
    repin_all_lockfiles,
    run_cargo_vet_check_report,
    sync_workspace_version,
    vet_rust_dependencies,
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

    def test_repin_all_lockfiles_invokes_bazel(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            with patch("arena_version.subprocess.run") as run:
                repin_all_lockfiles(root)
                self.assertEqual(run.call_count, 5)

    def test_repin_all_lockfiles_passes_bazel_config(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            with patch.dict(os.environ, {"ARENA_BAZEL_CONFIG": "ci"}):
                with patch("arena_version.subprocess.run") as run:
                    repin_all_lockfiles(root)
                    for call in run.call_args_list:
                        self.assertIn("--config=ci", call.args[0])

    def test_audit_arena_ffi_binary_toolsInstalled_skipsInstall(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            with (
                patch("arena_version.subprocess.run") as run,
                patch("arena_version.shutil.which", return_value="/usr/bin/cargo-audit"),
            ):
                audit_arena_ffi_binary(root)
                self.assertEqual(run.call_count, 2)

    def test_audit_arena_ffi_binary_toolsMissing_installsFirst(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            with (
                patch("arena_version.subprocess.run") as run,
                patch("arena_version.shutil.which", side_effect=_cargo_only),
            ):
                audit_arena_ffi_binary(root)
                self.assertEqual(run.call_count, 3)
                self.assertIn("install", run.call_args_list[0].args[0])

    def test_vet_rust_dependencies_toolInstalled_skipsInstall(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            with (
                patch("arena_version.subprocess.run") as run,
                patch("arena_version.shutil.which", return_value="/usr/bin/cargo-vet"),
            ):
                vet_rust_dependencies(root)
                self.assertEqual(run.call_count, 1)
                self.assertEqual(run.call_args_list[0].args[0], ["cargo", "vet"])

    def test_vet_rust_dependencies_toolMissing_installsFirst(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            with (
                patch("arena_version.subprocess.run") as run,
                patch("arena_version.shutil.which", side_effect=_cargo_only),
            ):
                vet_rust_dependencies(root)
                self.assertEqual(run.call_count, 2)
                self.assertIn("install", run.call_args_list[0].args[0])


class RequireCargoTest(unittest.TestCase):
    def test_audit_arena_ffi_binary_cargo_missing_raises_actionable_error(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            with (
                patch("arena_version.subprocess.run") as run,
                patch("arena_version.shutil.which", return_value=None),
            ):
                with self.assertRaises(SystemExit) as raised:
                    audit_arena_ffi_binary(root)
                self.assertIn("cargo not found on PATH", str(raised.exception))
                self.assertIn("rustup.rs", str(raised.exception))
                run.assert_not_called()

    def test_vet_rust_dependencies_cargo_missing_raises_actionable_error(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            with (
                patch("arena_version.subprocess.run") as run,
                patch("arena_version.shutil.which", return_value=None),
            ):
                with self.assertRaises(SystemExit) as raised:
                    vet_rust_dependencies(root)
                self.assertIn("cargo not found on PATH", str(raised.exception))
                run.assert_not_called()

    def test_vet_rust_dependencies_cargo_present_does_not_raise(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            with (
                patch("arena_version.subprocess.run"),
                patch("arena_version.shutil.which", side_effect=_cargo_only),
            ):
                vet_rust_dependencies(root)


def _cargo_only(tool: str):
    return "/usr/bin/cargo" if tool == "cargo" else None


def _vet_report(audited: int, exempted: int) -> dict:
    return {
        "conclusion": "success",
        "vetted_fully": [{"name": f"audited{i}", "version": "1.0.0"} for i in range(audited)],
        "vetted_partially": [],
        "vetted_with_exemptions": [
            {"name": f"exempted{i}", "version": "1.0.0"} for i in range(exempted)
        ],
    }


class RunCargoVetCheckReportTest(unittest.TestCase):
    def test_run_cargo_vet_check_report_success_returnsParsedJson(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            fake_result = subprocess.CompletedProcess(
                args=["cargo", "vet", "check", "--output-format=json"],
                returncode=0,
                stdout='{"conclusion": "success", "vetted_fully": []}',
            )
            with patch("arena_version.subprocess.run", return_value=fake_result) as run:
                report = run_cargo_vet_check_report(root)
                self.assertEqual(report, {"conclusion": "success", "vetted_fully": []})
                self.assertEqual(
                    run.call_args_list[0].args[0],
                    ["cargo", "vet", "check", "--output-format=json"],
                )


class CheckCargoVetWatermarksTest(unittest.TestCase):
    def test_check_cargo_vet_watermarks_countsUnchanged_doesNotRaise(self) -> None:
        report = _vet_report(
            CARGO_VET_AUDITED_PACKAGE_COUNT_WATERMARK, CARGO_VET_EXEMPTED_PACKAGE_COUNT_WATERMARK
        )
        check_cargo_vet_watermarks(report)

    def test_check_cargo_vet_watermarks_auditedCountIncreased_raises(self) -> None:
        report = _vet_report(
            CARGO_VET_AUDITED_PACKAGE_COUNT_WATERMARK + 1,
            CARGO_VET_EXEMPTED_PACKAGE_COUNT_WATERMARK,
        )
        with self.assertRaises(SystemExit):
            check_cargo_vet_watermarks(report)

    def test_check_cargo_vet_watermarks_exemptedCountIncreased_raises(self) -> None:
        report = _vet_report(
            CARGO_VET_AUDITED_PACKAGE_COUNT_WATERMARK,
            CARGO_VET_EXEMPTED_PACKAGE_COUNT_WATERMARK + 1,
        )
        with self.assertRaises(SystemExit):
            check_cargo_vet_watermarks(report)

    def test_check_cargo_vet_watermarks_exemptedCountDecreased_raises(self) -> None:
        report = _vet_report(
            CARGO_VET_AUDITED_PACKAGE_COUNT_WATERMARK,
            CARGO_VET_EXEMPTED_PACKAGE_COUNT_WATERMARK - 1,
        )
        with self.assertRaises(SystemExit):
            check_cargo_vet_watermarks(report)


if __name__ == "__main__":
    unittest.main()
