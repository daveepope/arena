from __future__ import annotations

import os
import re
import tomllib
from pathlib import Path

RUST_CONST_ID = re.compile(r"^[a-z][a-z0-9_]*$")

BUILDER_IMAGE_IDS = frozenset(
    {
        "postgres",
        "http",
        "mssql",
        "localstack",
        "kafka_apache",
        "kafka_confluent",
    }
)


def _repo_root() -> Path:
    ws = os.environ.get("BUILD_WORKSPACE_DIRECTORY")
    if ws:
        return Path(ws)
    return Path(__file__).resolve().parent.parent


def rust_string_literal(value: str) -> str:
    escaped = value.replace("\\", "\\\\").replace('"', '\\"')
    return f'"{escaped}"'


def validate_image_id(image_id: str) -> None:
    if not RUST_CONST_ID.fullmatch(image_id):
        raise ValueError(
            f"image id {image_id!r} must match {RUST_CONST_ID.pattern} "
            "to generate a valid Rust const name"
        )


def validate_container_default_entries(entries: list[dict[str, str]]) -> None:
    seen: set[str] = set()
    for entry in entries:
        image_id = entry["id"]
        validate_image_id(image_id)
        if image_id in seen:
            raise ValueError(f"duplicate image id: {image_id}")
        seen.add(image_id)
        for field in ("image", "tag"):
            if not entry[field]:
                raise ValueError(f"image {image_id!r} missing {field}")


def load_container_defaults(
    root: Path | None = None,
    toml_path: Path | None = None,
) -> list[dict[str, str]]:
    base = toml_path or (root or _repo_root()) / "container_defaults.toml"
    data = tomllib.loads(base.read_text(encoding="utf-8"))
    images = data.get("image", [])
    out: list[dict[str, str]] = []
    for entry in images:
        out.append(
            {
                "id": str(entry["id"]),
                "image": str(entry["image"]),
                "tag": str(entry["tag"]),
            }
        )
    out = sorted(out, key=lambda row: row["id"])
    validate_container_default_entries(out)
    return out


def image_refs(root: Path | None = None) -> list[str]:
    return [
        f"{entry['image']}:{entry['tag']}"
        for entry in load_container_defaults(root)
    ]


def render_default_images_rs(entries: list[dict[str, str]]) -> str:
    validate_container_default_entries(entries)
    const_blocks: list[str] = []
    const_names: list[str] = []
    for entry in entries:
        const_name = entry["id"].upper()
        const_names.append(const_name)
        const_blocks.append(
            "\n".join(
                [
                    f"pub const {const_name}: DefaultImageRef = DefaultImageRef {{",
                    f"    id: {rust_string_literal(entry['id'])},",
                    f"    image: {rust_string_literal(entry['image'])},",
                    f"    tag: {rust_string_literal(entry['tag'])},",
                    "};",
                ]
            )
        )
    const_section = "\n\n".join(const_blocks)
    all_entries = ",\n    ".join(const_names)
    return (
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]\n"
        "pub struct DefaultImageRef {\n"
        "    pub id: &'static str,\n"
        "    pub image: &'static str,\n"
        "    pub tag: &'static str,\n"
        "}\n"
        "\n"
        "impl DefaultImageRef {\n"
        "    pub const fn image_ref(self) -> (&'static str, &'static str) {\n"
        "        (self.image, self.tag)\n"
        "    }\n"
        "}\n"
        "\n"
        f"{const_section}\n"
        "\n"
        "pub const ALL: &[DefaultImageRef] = &[\n"
        f"    {all_entries},\n"
        "];\n"
    )
