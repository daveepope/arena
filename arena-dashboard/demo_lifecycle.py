#!/usr/bin/env python3

from __future__ import annotations

import json
import os
import sys
import termios
import threading
import time
import tty
import urllib.error
import urllib.request

STEP_DELAY_SECONDS = 0.25

# A node is {"id": str, "children": [node, ...], "dependency_ids": [str, ...]}.
# "children" and "dependency_ids" are optional; dependency_ids only makes
# sense on a component node (see layout.ts's depByRawId resolution).

TOPOLOGY_A = {
    "dependencies": [
        {"id": "mssql"},
        {"id": "http-dep-1"},
        {"id": "kafka"},
        {"id": "postgres", "children": [{"id": "http-dep-2"}]},
    ],
    "components": [
        {
            "id": "parent-component",
            "dependency_ids": ["mssql", "http-dep-1"],
            "children": [
                {"id": "child-component", "dependency_ids": ["kafka", "postgres"]},
            ],
        },
    ],
}

TOPOLOGY_B = {
    "dependencies": [
        {"id": "redis"},
        {"id": "s3-bucket"},
        {"id": "rabbitmq", "children": [{"id": "dead-letter-queue"}]},
    ],
    "components": [
        {
            "id": "api-gateway",
            "dependency_ids": ["redis", "s3-bucket"],
            "children": [
                {"id": "worker-pool", "dependency_ids": ["rabbitmq"]},
            ],
        },
    ],
}

ARENAS = {
    "arena-topology-a": TOPOLOGY_A,
    "arena-topology-b": TOPOLOGY_B,
}


def _postorder(nodes: list[dict]) -> list[str]:
    """Children before their own parent, depth-first, siblings in order."""
    out: list[str] = []
    for node in nodes:
        out.extend(_postorder(node.get("children", [])))
        out.append(node["id"])
    return out


def _subject(node: dict, states: dict[str, str]) -> dict:
    subject = {
        "id": node["id"],
        "state": states[node["id"]],
        "faults": [],
        "children": [_subject(c, states) for c in node.get("children", [])],
    }
    if "dependency_ids" in node:
        subject["dependency_ids"] = node["dependency_ids"]
    return subject


class DemoState:
    def __init__(self, arena_id: str, topology: dict):
        self.arena_id = arena_id
        self.topology = topology
        self.start = time.monotonic()
        self.arena = "arena_created"
        self.subjects: dict[str, str] = {
            id_: "not_started"
            for id_ in _postorder(topology["dependencies"]) + _postorder(topology["components"])
        }

    def at(self) -> str:
        return f"t+{time.monotonic() - self.start:.2f}s"

    def snapshot(self) -> dict:
        return {
            "id": self.arena_id,
            "state": self.arena,
            "at": self.at(),
            "dependencies": [_subject(n, self.subjects) for n in self.topology["dependencies"]],
            "components": [_subject(n, self.subjects) for n in self.topology["components"]],
            "faults": [],
        }


def _send(url: str, state: DemoState, label: str) -> None:
    body = json.dumps(state.snapshot()).encode("utf-8")
    req = urllib.request.Request(
        url, data=body, headers={"Content-Type": "application/json"}, method="POST"
    )
    try:
        with urllib.request.urlopen(req, timeout=5) as resp:
            print(f"[{state.arena_id} {state.at()}] {label} ({resp.status})")
    except urllib.error.URLError as exc:
        print(f"[{state.arena_id} {state.at()}] {label} - ingest failed: {exc}", file=sys.stderr)


def _step_arena(url: str, state: DemoState, value: str) -> None:
    state.arena = value
    _send(url, state, f"arena: {value}")
    time.sleep(STEP_DELAY_SECONDS)


def _step_subject(url: str, state: DemoState, id_: str, value: str) -> None:
    state.subjects[id_] = value
    _send(url, state, f"{id_}: {value}")
    time.sleep(STEP_DELAY_SECONDS)


def _step_subjects(url: str, state: DemoState, ids: list[str], value: str, label: str) -> None:
    for id_ in ids:
        state.subjects[id_] = value
    _send(url, state, label)
    time.sleep(STEP_DELAY_SECONDS)


def _wait_for_keypress() -> None:
    if not sys.stdin.isatty():
        sys.stdin.readline()
        return
    fd = sys.stdin.fileno()
    old = termios.tcgetattr(fd)
    try:
        tty.setraw(fd)
        sys.stdin.read(1)
    finally:
        termios.tcsetattr(fd, termios.TCSADRAIN, old)


def run_arena(arena_id: str, topology: dict, url: str, teardown_event: threading.Event) -> None:
    state = DemoState(arena_id, topology)

    _send(url, state, "arena: arena_created")
    time.sleep(STEP_DELAY_SECONDS)

    _step_arena(url, state, "arena_starting")
    _step_arena(url, state, "dependencies_starting")

    # Match::start_dependencies() (arena/src/matches.rs) starts every
    # top-level dependency concurrently via join_all. Leaves (no children)
    # and nested children have nothing blocking them, so they all transition
    # together within a single snapshot; a composite top-level dependency
    # (one with children, e.g. postgres/rabbitmq) only starts once its own
    # children are ready, as its own separate step.
    deps = topology["dependencies"]
    composites = [n for n in deps if n.get("children")]
    concurrent_ids = [n["id"] for n in deps if not n.get("children")]
    for c in composites:
        concurrent_ids.extend(_postorder(c["children"]))

    _step_subjects(url, state, concurrent_ids, "starting", "dependencies: starting (concurrent)")
    _step_subjects(url, state, concurrent_ids, "readiness_check", "dependencies: readiness_check (concurrent)")
    _step_subjects(url, state, concurrent_ids, "started", "dependencies: started (concurrent)")

    for c in composites:
        _step_subject(url, state, c["id"], "starting")
        _step_subject(url, state, c["id"], "readiness_check")
        _step_subject(url, state, c["id"], "started")

    _step_arena(url, state, "dependencies_started")
    _step_arena(url, state, "playbooks_running")
    _step_arena(url, state, "playbooks_complete")
    _step_arena(url, state, "components_starting")

    for id_ in _postorder(topology["components"]):
        _step_subject(url, state, id_, "starting")
        _step_subject(url, state, id_, "readiness_check")
        _step_subject(url, state, id_, "started")

    _step_arena(url, state, "components_started")
    _step_arena(url, state, "arena_open")

    teardown_event.wait()

    _step_arena(url, state, "arena_closing")
    _step_arena(url, state, "components_stopping")

    for id_ in reversed(_postorder(topology["components"])):
        _step_subject(url, state, id_, "stopping")
        _step_subject(url, state, id_, "stopped")

    _step_arena(url, state, "components_stopped")
    _step_arena(url, state, "dependencies_stopping")

    # Composites stop before releasing their nested children, same
    # structural relationship as on start, just reversed.
    for c in composites:
        _step_subject(url, state, c["id"], "stopping")
        _step_subject(url, state, c["id"], "stopped")

    _step_subjects(url, state, concurrent_ids, "stopping", "dependencies: stopping (concurrent)")
    _step_subjects(url, state, concurrent_ids, "stopped", "dependencies: stopped (concurrent)")

    _step_arena(url, state, "dependencies_stopped")
    _step_arena(url, state, "arena_teardown")
    _step_arena(url, state, "arena_closed")


def main() -> int:
    base = os.environ.get("ARENA_DASHBOARD_URL", "http://127.0.0.1:4873").rstrip("/")
    url = f"{base}/ingest"

    print(f"Posting demo lifecycle events for {len(ARENAS)} arenas to {url}")
    teardown_event = threading.Event()
    threads = [
        threading.Thread(target=run_arena, args=(arena_id, topology, url, teardown_event), name=arena_id)
        for arena_id, topology in ARENAS.items()
    ]
    for t in threads:
        t.start()

    print("\nAll arenas open. Press any key to tear down...")
    _wait_for_keypress()
    teardown_event.set()

    for t in threads:
        t.join()

    print("\nTeardown complete.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
