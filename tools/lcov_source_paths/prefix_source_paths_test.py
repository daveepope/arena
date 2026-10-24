import tempfile
import unittest
from pathlib import Path

from prefix_source_paths import main, prefix_source_paths

_LCOV = """TN:
SF:src/store.ts
DA:1,1
LH:1
LF:1
end_of_record
"""


class PrefixSourcePathsTest(unittest.TestCase):
    def test_relative_source_path_gains_prefix(self):
        result = prefix_source_paths(_LCOV, "arena-dashboard/web/")

        self.assertIn("SF:arena-dashboard/web/src/store.ts\n", result)

    def test_prefix_without_trailing_slash_adds_separator(self):
        result = prefix_source_paths(_LCOV, "arena-dashboard/web")

        self.assertIn("SF:arena-dashboard/web/src/store.ts\n", result)

    def test_already_prefixed_path_unchanged(self):
        already = "SF:arena-dashboard/web/src/store.ts\nend_of_record\n"

        result = prefix_source_paths(already, "arena-dashboard/web/")

        self.assertEqual(already, result)

    def test_absolute_source_path_unchanged(self):
        absolute = "SF:/abs/src/store.ts\nend_of_record\n"

        result = prefix_source_paths(absolute, "arena-dashboard/web/")

        self.assertEqual(absolute, result)

    def test_empty_prefix_returns_input(self):
        self.assertEqual(_LCOV, prefix_source_paths(_LCOV, ""))

    def test_empty_input_returns_empty(self):
        self.assertEqual("", prefix_source_paths("", "arena-dashboard/web/"))

    def test_non_source_lines_preserved_in_order(self):
        result = prefix_source_paths(_LCOV, "arena-dashboard/web/")

        self.assertEqual(
            ["TN:", "SF:arena-dashboard/web/src/store.ts", "DA:1,1", "LH:1", "LF:1", "end_of_record"],
            result.splitlines(),
        )

    def test_every_source_record_prefixed(self):
        many = "SF:src/a.ts\nend_of_record\nSF:src/b.ts\nend_of_record\n"

        result = prefix_source_paths(many, "arena-dashboard/web/")

        for path in ["arena-dashboard/web/src/a.ts", "arena-dashboard/web/src/b.ts"]:
            with self.subTest(path=path):
                self.assertIn("SF:%s\n" % path, result)

    def test_main_writes_prefixed_file(self):
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory) / "lcov.info"
            destination = Path(directory) / "prefixed.info"
            source.write_text(_LCOV)

            exit_code = main(["prefix_source_paths", "arena-dashboard/web/", str(source), str(destination)])

            self.assertEqual(0, exit_code)
            self.assertIn("SF:arena-dashboard/web/src/store.ts\n", destination.read_text())


if __name__ == "__main__":
    unittest.main()
