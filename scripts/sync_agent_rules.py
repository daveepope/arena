#!/usr/bin/env python3
"""Regenerate CLAUDE.md and .cursor/rules/arena-agent.mdc from AGENTS.md."""

from __future__ import annotations

import os
import sys
from pathlib import Path


def _repo_root() -> Path:
    ws = os.environ.get("BUILD_WORKSPACE_DIRECTORY")
    if ws:
        return Path(ws)
    return Path(__file__).resolve().parent.parent


def main() -> int:
    root = _repo_root()
    agents = root / "AGENTS.md"
    if not agents.is_file():
        print(f"error: missing {agents}", file=sys.stderr)
        return 1

    text = agents.read_text(encoding="utf-8")

    claude = root / "CLAUDE.md"
    claude.write_text(text, encoding="utf-8", newline="\n")
    print(f"wrote {claude.relative_to(root)}")

    mdc = root / ".cursor" / "rules" / "arena-agent.mdc"
    mdc.parent.mkdir(parents=True, exist_ok=True)
    front = (
        "---\n"
        "description: Arena — agent instructions (generated from AGENTS.md; do not edit by hand)\n"
        "alwaysApply: true\n"
        "---\n\n"
    )
    mdc.write_text(front + text, encoding="utf-8", newline="\n")
    print(f"wrote {mdc.relative_to(root)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
