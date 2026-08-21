import sys
import unittest
from datetime import datetime, timedelta, timezone
from pathlib import Path
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parent))

from check_dependency_release_age import (
    check_release_ages,
    local_cargo_crates_from_bazel_lock,
    parse_cargo_bazel_lock,
    parse_module_bazel,
    parse_requirements_lock,
)


class ParseCargoBazelLockTest(unittest.TestCase):
    def test_parse_cargo_bazel_lock_known_crate_returns_version(self) -> None:
        text = """
{
  "crates": {
    "serde 1.0.210": {
      "name": "serde",
      "version": "1.0.210"
    }
  }
}
"""
        self.assertEqual(
            parse_cargo_bazel_lock(text),
            {("cargo", "serde", "1.0.210")},
        )


class ParseRequirementsLockTest(unittest.TestCase):
    def test_parse_requirements_lock_pinned_line_returns_version(self) -> None:
        text = """
pytest==9.0.3
    # via -r arena-pytest/requirements.txt
requests==2.34.2
"""
        self.assertEqual(
            parse_requirements_lock(text),
            {("pip", "pytest", "9.0.3"), ("pip", "requests", "2.34.2")},
        )


class ParseModuleBazelTest(unittest.TestCase):
    def test_parse_module_bazel_coords_returns_maven_and_bcr(self) -> None:
        text = """
bazel_dep(name = "rules_python", version = "0.34.0")
arena_java_maven.install(
    artifacts = [
        "com.fasterxml.jackson.core:jackson-databind:2.18.2",
    ],
)
"""
        self.assertEqual(
            parse_module_bazel(text),
            {
                ("bcr", "rules_python", "0.34.0"),
                ("maven", "com.fasterxml.jackson.core:jackson-databind", "2.18.2"),
            },
        )

    def test_parse_module_bazel_nuget_archive_returns_nuget(self) -> None:
        text = """
nuget_archive(
    name = "newtonsoft_json",
    id = "Newtonsoft.Json",
    sources = ["https://api.nuget.org/v3/index.json"],
    version = "13.0.4",
)
"""
        self.assertEqual(
            parse_module_bazel(text),
            {("nuget", "Newtonsoft.Json", "13.0.4")},
        )


class LocalCargoCratesFromBazelLockTest(unittest.TestCase):
    def test_local_cargo_crates_null_package_url_returns_name_version(self) -> None:
        text = """
{
  "crates": {
    "arena 1.3.0": {
      "name": "arena",
      "version": "1.3.0",
      "package_url": null
    },
    "serde 1.0.228": {
      "name": "serde",
      "version": "1.0.228",
      "package_url": "https://github.com/serde-rs/serde"
    }
  }
}
"""
        self.assertEqual(
            local_cargo_crates_from_bazel_lock(text),
            {("arena", "1.3.0")},
        )


class CheckReleaseAgesTest(unittest.TestCase):
    def test_maven_undeterminable_publish_time_fails(self) -> None:
        with (
            mock.patch(
                "check_dependency_release_age._changed_watched_files",
                return_value=["MODULE.bazel"],
            ),
            mock.patch(
                "check_dependency_release_age._collect_new_versions",
                return_value={("maven", "org.postgresql:postgresql", "42.7.12")},
            ),
            mock.patch(
                "check_dependency_release_age._published_at",
                return_value=None,
            ),
            mock.patch(
                "check_dependency_release_age._read_file_at_ref",
                return_value="",
            ),
        ):
            failures = check_release_ages(
                Path("."),
                "origin/master",
                minimum_age=timedelta(days=3),
                now=datetime.now(timezone.utc),
            )

        self.assertEqual(len(failures), 1)
        self.assertIn("maven org.postgresql:postgresql 42.7.12", failures[0])

    def test_bcr_undeterminable_publish_time_fails(self) -> None:
        with (
            mock.patch(
                "check_dependency_release_age._changed_watched_files",
                return_value=["MODULE.bazel"],
            ),
            mock.patch(
                "check_dependency_release_age._collect_new_versions",
                return_value={("bcr", "rules_python", "0.34.0")},
            ),
            mock.patch(
                "check_dependency_release_age._published_at",
                return_value=None,
            ),
            mock.patch(
                "check_dependency_release_age._read_file_at_ref",
                return_value="",
            ),
        ):
            failures = check_release_ages(
                Path("."),
                "origin/master",
                minimum_age=timedelta(days=3),
                now=datetime.now(timezone.utc),
            )

        self.assertEqual(len(failures), 1)
        self.assertIn("bcr rules_python 0.34.0", failures[0])

    def test_nuget_undeterminable_publish_time_fails(self) -> None:
        with (
            mock.patch(
                "check_dependency_release_age._changed_watched_files",
                return_value=["MODULE.bazel"],
            ),
            mock.patch(
                "check_dependency_release_age._collect_new_versions",
                return_value={("nuget", "Newtonsoft.Json", "13.0.4")},
            ),
            mock.patch(
                "check_dependency_release_age._published_at",
                return_value=None,
            ),
            mock.patch(
                "check_dependency_release_age._read_file_at_ref",
                return_value="",
            ),
        ):
            failures = check_release_ages(
                Path("."),
                "origin/master",
                minimum_age=timedelta(days=3),
                now=datetime.now(timezone.utc),
            )

        self.assertEqual(len(failures), 1)
        self.assertIn("nuget Newtonsoft.Json 13.0.4", failures[0])

    def test_nuget_recent_publish_time_fails(self) -> None:
        with (
            mock.patch(
                "check_dependency_release_age._changed_watched_files",
                return_value=["MODULE.bazel"],
            ),
            mock.patch(
                "check_dependency_release_age._collect_new_versions",
                return_value={("nuget", "Newtonsoft.Json", "13.0.4")},
            ),
            mock.patch(
                "check_dependency_release_age._published_at",
                return_value=datetime.now(timezone.utc) - timedelta(hours=1),
            ),
            mock.patch(
                "check_dependency_release_age._read_file_at_ref",
                return_value="",
            ),
        ):
            failures = check_release_ages(
                Path("."),
                "origin/master",
                minimum_age=timedelta(days=3),
                now=datetime.now(timezone.utc),
            )

        self.assertEqual(len(failures), 1)
        self.assertIn("nuget Newtonsoft.Json 13.0.4", failures[0])


if __name__ == "__main__":
    unittest.main()
