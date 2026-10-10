from __future__ import annotations

import os
import queue
import subprocess
import threading
import time
from typing import Callable, Optional

import pytest
import requests


def _find_dashboard_server_executable() -> str:
    pkg_dir = os.path.dirname(os.path.abspath(__file__))
    for name in (
        "arena-dashboard-server-linux",
        "arena-dashboard-server-macos",
        "arena-dashboard-server.exe",
    ):
        p = os.path.join(pkg_dir, name)
        if os.path.isfile(p):
            return p

    runfiles_dir = os.environ.get("RUNFILES_DIR")
    if runfiles_dir:
        for base in ("_main", "arena", ""):
            p = os.path.join(runfiles_dir, base, "arena-dashboard", "server")
            if os.path.isfile(p):
                return p
    return ""


def _forwarder(dashboard_url: str) -> Callable[[str], None]:
    q: "queue.Queue[str]" = queue.Queue()

    def _drain() -> None:
        while True:
            document = q.get()
            try:
                requests.post(
                    f"{dashboard_url.rstrip('/')}/ingest",
                    data=document,
                    headers={"Content-Type": "application/json"},
                    timeout=2,
                )
            except requests.RequestException:
                pass

    threading.Thread(target=_drain, daemon=True).start()
    return q.put_nowait


@pytest.fixture(scope="session")
def arena_dashboard_server() -> Optional[Callable[[str], None]]:
    """Opt-in (ARENA_VISUALIZE=1, or ARENA_DASHBOARD_URL pointing at an
    already-running server): starts arena-dashboard's server before the
    arena under test opens, yielding a ready-to-use ClosedArena.observe()
    callback - or None when visualization was not requested. The server is
    left running after the arena closes (and after this process exits) so
    results stay visible; a later run's own start attempt just finds it
    already there (arena-dashboard's own AddrInUse handling) and reuses it."""
    dashboard_url = os.environ.get("ARENA_DASHBOARD_URL")
    if os.environ.get("ARENA_VISUALIZE") != "1" and not dashboard_url:
        yield None
        return

    if not dashboard_url:
        exe = _find_dashboard_server_executable()
        if not exe:
            pytest.fail(
                "ARENA_VISUALIZE=1 but arena-dashboard's server was not found in "
                "runfiles - depend on //arena-dashboard:server"
            )
        port = int(os.environ.get("ARENA_DASHBOARD_PORT", "4873"))
        dashboard_url = f"http://127.0.0.1:{port}"
        subprocess.Popen(
            [exe],
            env={**os.environ, "ARENA_DASHBOARD_PORT": str(port)},
            start_new_session=True,
        )

        deadline = time.monotonic() + 10.0
        ready = False
        while time.monotonic() < deadline:
            try:
                if requests.get(dashboard_url, timeout=0.5).status_code == 200:
                    ready = True
                    break
            except requests.RequestException:
                pass
            time.sleep(0.1)
        if not ready:
            pytest.fail(f"arena-dashboard did not become ready on {dashboard_url} within 10s")

        print(f"\narena-dashboard running at {dashboard_url} - open it in a browser now\n")

    yield _forwarder(dashboard_url)
