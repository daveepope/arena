from __future__ import annotations

import os
import unittest
from pathlib import Path

from container_defaults import image_refs, load_container_defaults


class ContainerDefaultsTest(unittest.TestCase):
    def _root(self) -> Path:
        ws = os.environ.get("BUILD_WORKSPACE_DIRECTORY")
        if ws:
            return Path(ws)
        return Path(__file__).resolve().parent.parent

    def test_load_container_defaults_returns_six_images(self) -> None:
        rows = load_container_defaults(self._root())
        self.assertEqual(len(rows), 6)
        self.assertEqual(rows[0]["id"], "http")

    def test_image_refs_formats_image_colon_tag(self) -> None:
        refs = image_refs(self._root())
        self.assertIn("postgres:17", refs)


if __name__ == "__main__":
    unittest.main()
