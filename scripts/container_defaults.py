from __future__ import annotations

import os
import tomllib
from pathlib import Path


def _repo_root() -> Path:
    ws = os.environ.get("BUILD_WORKSPACE_DIRECTORY")
    if ws:
        return Path(ws)
    return Path(__file__).resolve().parent.parent


def load_container_defaults(root: Path | None = None) -> list[dict[str, str]]:
    base = root or _repo_root()
    data = tomllib.loads((base / "container_defaults.toml").read_text(encoding="utf-8"))
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
    return sorted(out, key=lambda row: row["id"])


def image_refs(root: Path | None = None) -> list[str]:
    return [
        f"{entry['image']}:{entry['tag']}"
        for entry in load_container_defaults(root)
    ]
