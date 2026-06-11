import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from check_dependency_release_age import (
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


if __name__ == "__main__":
    unittest.main()
