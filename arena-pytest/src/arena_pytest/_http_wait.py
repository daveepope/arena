"""Internal HTTP polling helper for tests (not public API)."""

import time

import requests


def wait_for_http_ready(url: str, timeout_sec: float = 30, poll_interval_sec: float = 0.1) -> None:
    deadline = time.monotonic() + timeout_sec
    while time.monotonic() < deadline:
        try:
            requests.get(url, timeout=5)
            return
        except Exception:
            pass
        time.sleep(poll_interval_sec)
    raise TimeoutError(f"HTTP readiness check timed out after {timeout_sec}s: {url}")
