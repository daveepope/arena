import asyncio
import json
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

import pytest

from arena_pytest.closed_arena import ClosedArena
from arena_pytest.ffi._ffi import (
    ArenaBindingError,
    close_arena,
    dashboard_ensure_running,
    load_ffi,
    open_arena,
    register_dashboard_observer,
    unregister_lifecycle_observer,
)


@pytest.fixture
def arena_ffi():
    ffi = load_ffi()
    if ffi is None:
        pytest.skip("arena shared library not found")
    return ffi


class _CaptureHandler(BaseHTTPRequestHandler):
    def do_POST(self) -> None:
        length = int(self.headers.get("Content-Length", "0"))
        body = self.rfile.read(length).decode("utf-8")
        self.server.bodies.append(body)
        self.send_response(200)
        self.end_headers()

    def log_message(self, *args) -> None:
        pass


class _CaptureServer:
    def __init__(self):
        self.bodies: list[str] = []
        self._server = ThreadingHTTPServer(("127.0.0.1", 0), _CaptureHandler)
        self._server.bodies = self.bodies
        self._thread = threading.Thread(target=self._server.serve_forever, daemon=True)
        self._thread.start()

    def url(self) -> str:
        host, port = self._server.server_address
        return f"http://{host}:{port}"

    def close(self) -> None:
        self._server.shutdown()
        self._thread.join()


@pytest.fixture
def capture_server():
    server = _CaptureServer()
    try:
        yield server
    finally:
        server.close()


def test_dashboard_ensure_running_no_server_and_no_search_dir_raises_binding_error(arena_ffi):
    with pytest.raises(ArenaBindingError):
        dashboard_ensure_running(arena_ffi, 1)


def test_register_dashboard_observer_open_and_close_forwards_state_via_http(
    arena_ffi, capture_server
):
    token = register_dashboard_observer(arena_ffi, capture_server.url())
    try:
        handle = open_arena(arena_ffi, b"py-dashboard-forward")
        close_arena(arena_ffi, handle)
    finally:
        unregister_lifecycle_observer(arena_ffi, token)

    documents = [json.loads(b) for b in capture_server.bodies]
    assert documents, "no states were forwarded via http"
    assert any(d.get("id") == "py-dashboard-forward" for d in documents)


def test_closed_arena_with_lifecycle_dashboard_open_and_close_forwards_state_via_http(
    arena_ffi, capture_server
):
    closed = ClosedArena("py-visualize-forward", []).with_lifecycle_dashboard(
        capture_server.url()
    )

    async def run():
        opened = await closed.open()
        await opened.close()

    asyncio.run(run())

    documents = [json.loads(b) for b in capture_server.bodies]
    assert documents, "no states were forwarded via http"
    assert any(d.get("id") == "py-visualize-forward" for d in documents)


def test_closed_arena_with_lifecycle_dashboard_dashboard_unavailable_opens_arena_anyway(
    arena_ffi,
):
    closed = ClosedArena("py-visualize-unavailable", []).with_lifecycle_dashboard(
        port=1, search_dir="/nonexistent-search-dir"
    )

    async def run():
        opened = await closed.open()
        await opened.close()

    asyncio.run(run())


def test_closed_arena_with_lifecycle_dashboard_close_unregisters_forwarding(
    arena_ffi, capture_server
):
    closed = ClosedArena("py-visualize-teardown", []).with_lifecycle_dashboard(
        capture_server.url()
    )

    async def run():
        opened = await closed.open()
        await opened.close()

    asyncio.run(run())
    count_after_first_close = len(capture_server.bodies)

    handle = open_arena(arena_ffi, b"py-visualize-teardown-unrelated")
    close_arena(arena_ffi, handle)

    assert len(capture_server.bodies) == count_after_first_close
