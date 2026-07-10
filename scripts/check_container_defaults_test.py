from __future__ import annotations

import os
import tempfile
import unittest
from pathlib import Path

from container_defaults import (
    BUILDER_IMAGE_IDS,
    load_container_defaults,
    render_default_images_rs,
    rust_string_literal,
    validate_image_id,
)


class ContainerDefaultsTest(unittest.TestCase):
    def _root(self) -> Path:
        ws = os.environ.get("BUILD_WORKSPACE_DIRECTORY")
        if ws:
            return Path(ws)
        return Path(__file__).resolve().parent.parent

    def test_load_container_defaults_includes_builder_ids(self) -> None:
        rows = load_container_defaults(self._root())
        loaded_ids = {row["id"] for row in rows}
        self.assertTrue(BUILDER_IMAGE_IDS.issubset(loaded_ids))

    def test_load_container_defaults_postgres_tag_is_17(self) -> None:
        rows = load_container_defaults(self._root())
        postgres = next(row for row in rows if row["id"] == "postgres")
        self.assertEqual(postgres["image"], "postgres")
        self.assertEqual(postgres["tag"], "17")

    def test_render_default_images_rs_matches_toml_entries(self) -> None:
        rows = load_container_defaults(self._root())
        rendered = render_default_images_rs(rows)
        for entry in rows:
            const_name = entry["id"].upper()
            self.assertIn(f"pub const {const_name}: DefaultImageRef", rendered)
            self.assertIn(f'id: {rust_string_literal(entry["id"])}', rendered)
            self.assertIn(f'image: {rust_string_literal(entry["image"])}', rendered)
            self.assertIn(f'tag: {rust_string_literal(entry["tag"])}', rendered)

    def test_render_default_images_rs_escapes_quotes(self) -> None:
        rows = [
            {
                "id": "http",
                "image": "example/repo",
                "tag": '2022-CU14"bad',
            }
        ]
        rendered = render_default_images_rs(rows)
        self.assertIn('tag: "2022-CU14\\"bad"', rendered)

    def test_validate_image_id_rejects_invalid_const_name(self) -> None:
        with self.assertRaises(ValueError):
            validate_image_id("kafka-apache")
        with self.assertRaises(ValueError):
            validate_image_id("123")

    def test_load_container_defaults_rejects_duplicate_ids(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            toml_path = Path(tmp) / "container_defaults.toml"
            toml_path.write_text(
                '[[image]]\nid = "http"\nimage = "a"\ntag = "1"\n'
                '[[image]]\nid = "http"\nimage = "b"\ntag = "2"\n',
                encoding="utf-8",
            )
            with self.assertRaises(ValueError):
                load_container_defaults(toml_path=toml_path)


if __name__ == "__main__":
    unittest.main()
