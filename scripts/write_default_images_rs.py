#!/usr/bin/env python3

from __future__ import annotations

import sys
from pathlib import Path

from container_defaults import load_container_defaults, render_default_images_rs


def main() -> int:
    toml_path = Path(sys.argv[1])
    out_path = Path(sys.argv[2])
    entries = load_container_defaults(toml_path=toml_path)
    out_path.write_text(render_default_images_rs(entries), encoding="utf-8")
    return 0


if __name__ == "__main__":
    sys.exit(main())
