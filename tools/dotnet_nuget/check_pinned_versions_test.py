from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

from check_pinned_versions import check_pinned_versions_match


class CheckPinnedVersionsMatchTest(unittest.TestCase):
    def _write_repo_files(self, root: Path, archive_version: str, pinned_version: str) -> None:
        (root / "MODULE.bazel").write_text(
            f"""
nuget_archive(
    name = "newtonsoft_json",
    id = "Newtonsoft.Json",
    sources = ["https://api.nuget.org/v3/index.json"],
    version = "{archive_version}",
)

nuget_archive(
    name = "ms_logging_abstractions",
    id = "Microsoft.Extensions.Logging.Abstractions",
    sources = ["https://api.nuget.org/v3/index.json"],
    version = "8.0.2",
)
""",
            encoding="utf-8",
        )
        (root / "tools/dotnet_nuget").mkdir(parents=True, exist_ok=True)
        (root / "tools/dotnet_nuget/pinned_dependency_versions.bzl").write_text(
            f'NEWTONSOFT_JSON_VERSION = "{pinned_version}"\n'
            'MS_LOGGING_ABSTRACTIONS_VERSION = "8.0.2"\n',
            encoding="utf-8",
        )

    def test_check_pinned_versions_match_alignedVersions_returnsNoMismatches(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self._write_repo_files(root, "13.0.4", "13.0.4")
            self.assertEqual(check_pinned_versions_match(root), [])

    def test_check_pinned_versions_match_driftedVersion_returnsMismatch(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self._write_repo_files(root, "13.0.5", "13.0.4")
            mismatches = check_pinned_versions_match(root)
            self.assertEqual(len(mismatches), 1)
            self.assertIn("newtonsoft_json", mismatches[0])
            self.assertIn("13.0.5", mismatches[0])
            self.assertIn("13.0.4", mismatches[0])


if __name__ == "__main__":
    unittest.main()
