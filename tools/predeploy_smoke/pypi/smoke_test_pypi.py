from __future__ import annotations

import subprocess
import sys
import tempfile
import venv
from pathlib import Path

_PROBE_SCRIPT = """\
import asyncio

from arena_pytest import ClosedArena, MatchBuilder


async def main():
    match = MatchBuilder("pypi-smoke-test-match").build()
    closed = ClosedArena("pypi-smoke-test-arena", [match])
    arena = await closed.open()
    print("SMOKE_TEST_ARENA_OPENED")
    await arena.close()
    print("SMOKE_TEST_ARENA_CLOSED")


asyncio.run(main())
"""


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: smoke_test_pypi.py <wheel-path>", file=sys.stderr)
        return 1
    wheel_path = Path(sys.argv[1]).resolve()
    if not wheel_path.is_file():
        print(f"smoke test: wheel not found at {wheel_path}", file=sys.stderr)
        return 1

    with tempfile.TemporaryDirectory(prefix="arena-pypi-smoke-") as workdir_str:
        workdir = Path(workdir_str)
        venv_dir = workdir / "venv"
        venv.EnvBuilder(with_pip=True, clear=True, symlinks=sys.platform != "win32").create(venv_dir)
        venv_python = venv_dir / ("Scripts/python.exe" if sys.platform == "win32" else "bin/python")

        subprocess.run(
            [str(venv_python), "-m", "pip", "install", "--quiet", str(wheel_path)],
            check=True,
        )

        probe_path = workdir / "probe.py"
        probe_path.write_text(_PROBE_SCRIPT)

        result = subprocess.run(
            [str(venv_python), str(probe_path)],
            capture_output=True,
            text=True,
        )
        sys.stdout.write(result.stdout)
        sys.stderr.write(result.stderr)

        if result.returncode != 0:
            print(f"smoke test FAILED: probe exited {result.returncode}", file=sys.stderr)
            return 1
        if "SMOKE_TEST_ARENA_OPENED" not in result.stdout or "SMOKE_TEST_ARENA_CLOSED" not in result.stdout:
            print("smoke test FAILED: did not see expected open/close markers in output", file=sys.stderr)
            return 1

    print(f"smoke test PASSED: {wheel_path.name} installed via pip, opened and closed a real arena")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
