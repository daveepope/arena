from __future__ import annotations

import asyncio
import os
import tempfile
import time
from typing import List

from arena_pytest import ClosedArena, ExecutableComponentBuilder, MatchBuilder

MATCH_NAME = "children-lifecycle-probe"


async def _open_and_close_with_children(marker_file: str) -> None:
    child = (
        ExecutableComponentBuilder("child")
        .with_executable_path("/bin/sh")
        .with_runtime_arg("flag", "-c")
        .with_runtime_arg("script", f"echo child >> {marker_file}")
        .build()
    )
    parent = (
        ExecutableComponentBuilder("parent")
        .with_executable_path("/bin/sh")
        .with_runtime_arg("flag", "-c")
        .with_runtime_arg("script", f"echo parent >> {marker_file}")
        .with_child_components([child])
        .build()
    )
    a_match = MatchBuilder(MATCH_NAME).add_component(parent).build()
    closed = ClosedArena(MATCH_NAME, [a_match])
    arena = await closed.open()
    try:
        assert arena is not None
    finally:
        await arena.close()


def _wait_for_marker_lines(marker_file: str, timeout_s: float = 5.0) -> List[str]:
    deadline = time.monotonic() + timeout_s
    while time.monotonic() < deadline:
        if os.path.exists(marker_file):
            with open(marker_file) as f:
                lines = [line.strip() for line in f.readlines()]
            if "child" in lines and "parent" in lines:
                return lines
        time.sleep(0.02)
    return []


def test_open_arena_with_child_component_starts_both_parent_and_child():
    with tempfile.TemporaryDirectory() as tmp_dir:
        marker_file = os.path.join(tmp_dir, "marker.txt")
        asyncio.run(_open_and_close_with_children(marker_file))
        lines = _wait_for_marker_lines(marker_file)
        assert "child" in lines
        assert "parent" in lines
