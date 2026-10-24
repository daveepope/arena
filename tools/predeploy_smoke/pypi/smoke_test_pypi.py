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

_DASHBOARD_PROBE_SCRIPT = """\
import asyncio
import socket
import urllib.request

from arena_pytest import ClosedArena, MatchBuilder


def pick_free_port():
    with socket.socket() as probe_socket:
        probe_socket.bind(("127.0.0.1", 0))
        return probe_socket.getsockname()[1]


def dashboard_is_serving(port):
    try:
        with urllib.request.urlopen(f"http://127.0.0.1:{port}/", timeout=10) as response:
            if response.status != 200:
                return False
            index = response.read().decode("utf-8")
        if "bundle.js" not in index:
            return False
        for asset in ("/bundle.js", "/styles.css"):
            with urllib.request.urlopen(f"http://127.0.0.1:{port}{asset}", timeout=10) as response:
                if response.status != 200 or not response.read():
                    return False
        return True
    except OSError:
        return False


def stop_dashboard(port):
    request = urllib.request.Request(f"http://127.0.0.1:{port}/shutdown", method="POST")
    try:
        urllib.request.urlopen(request, timeout=10).close()
    except OSError:
        pass


async def main():
    port = pick_free_port()
    match = MatchBuilder("pypi-dashboard-smoke-test-match").build()
    closed = ClosedArena(
        "pypi-dashboard-smoke-test-arena", [match]
    ).with_lifecycle_dashboard(port=port)
    arena = await closed.open()
    serving = dashboard_is_serving(port)
    await arena.close()
    stop_dashboard(port)
    if not serving:
        raise SystemExit(f"packaged dashboard server did not serve the dashboard on port {port}")
    print("SMOKE_TEST_DASHBOARD_SERVED")


asyncio.run(main())
"""


def _run_probe(venv_python, probe_path: Path, source: str, markers: list[str], label: str) -> bool:
    probe_path.write_text(source)
    result = subprocess.run([str(venv_python), str(probe_path)], capture_output=True, text=True)
    sys.stdout.write(result.stdout)
    sys.stderr.write(result.stderr)

    if result.returncode != 0:
        print(f"smoke test FAILED: {label} probe exited {result.returncode}", file=sys.stderr)
        return False
    missing = [marker for marker in markers if marker not in result.stdout]
    if missing:
        print(
            f"smoke test FAILED: {label} probe did not print {', '.join(missing)}",
            file=sys.stderr,
        )
        return False
    return True


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

        if not _run_probe(
            venv_python,
            workdir / "probe.py",
            _PROBE_SCRIPT,
            ["SMOKE_TEST_ARENA_OPENED", "SMOKE_TEST_ARENA_CLOSED"],
            "arena",
        ):
            return 1

        if not _run_probe(
            venv_python,
            workdir / "dashboard_probe.py",
            _DASHBOARD_PROBE_SCRIPT,
            ["SMOKE_TEST_DASHBOARD_SERVED"],
            "dashboard",
        ):
            return 1

    print(
        f"smoke test PASSED: {wheel_path.name} installed via pip, opened and closed a real arena "
        "and served the packaged dashboard server"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
