#!/usr/bin/env python3

from __future__ import annotations

import os
import tempfile
import unittest
from pathlib import Path

from run_osv_scan import _osv_scanner_canonical_repos, find_osv_scanner_bin


class _StubRunfiles:
    def __init__(self, mapping_path: str | None) -> None:
        self._mapping_path = mapping_path

    def Rlocation(self, path: str) -> str | None:
        if path == "_repo_mapping":
            return self._mapping_path
        return None


class OsvScannerCanonicalReposTest(unittest.TestCase):
    def test_canonical_repos_main_repo_entries_returns_canonical_names(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            mapping = Path(tmp) / "_repo_mapping"
            mapping.write_text(
                ",osv_scanner_linux_x86_64,rules~osv_scanner_linux_x86_64\n"
                ",osv_scanner_windows_x86_64,rules~osv_scanner_windows_x86_64\n"
                ",unrelated_repo,rules~unrelated_repo\n",
                encoding="utf-8",
            )
            self.assertEqual(
                _osv_scanner_canonical_repos(_StubRunfiles(str(mapping))),
                [
                    "rules~osv_scanner_linux_x86_64",
                    "rules~osv_scanner_windows_x86_64",
                ],
            )

    def test_canonical_repos_foreign_source_repo_returns_empty(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            mapping = Path(tmp) / "_repo_mapping"
            mapping.write_text(
                "other,osv_scanner_linux_x86_64,rules~osv_scanner_linux_x86_64\n",
                encoding="utf-8",
            )
            self.assertEqual(
                _osv_scanner_canonical_repos(_StubRunfiles(str(mapping))),
                [],
            )

    def test_canonical_repos_missing_mapping_returns_empty(self) -> None:
        self.assertEqual(_osv_scanner_canonical_repos(_StubRunfiles(None)), [])


class FindOsvScannerBinTest(unittest.TestCase):
    def test_find_bin_env_override_returns_that_path(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            binary = Path(tmp) / "osv-scanner"
            binary.write_text("", encoding="utf-8")
            previous = os.environ.get("ARENA_OSV_SCANNER_BIN")
            os.environ["ARENA_OSV_SCANNER_BIN"] = str(binary)
            try:
                self.assertEqual(find_osv_scanner_bin(), str(binary))
            finally:
                if previous is None:
                    del os.environ["ARENA_OSV_SCANNER_BIN"]
                else:
                    os.environ["ARENA_OSV_SCANNER_BIN"] = previous


if __name__ == "__main__":
    unittest.main()
