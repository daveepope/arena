from __future__ import annotations

import argparse
import subprocess
import sys
import tempfile
import venv
from pathlib import Path

_PROBE_SCRIPT = """\
import asyncio
import http.client as http_client
import math
import secrets
import statistics
import sys

import psycopg

from arena_pytest import (
    ActivePlaybook,
    ArenaLogLevel,
    ClosedArena,
    HttpDependencyBuilder,
    ManagedHttpPlaybook,
    ManagedPostgresPlaybook,
    MatchBuilder,
    PostgresDependencyBuilder,
    UnmanagedPlaybook,
)

VERSION = sys.argv[1]
ITERATIONS = int(sys.argv[2])

DB_SUFFIX = secrets.token_hex(4)
DB_NAME = f"bench_{DB_SUFFIX}"
DB_USER = f"bench_{DB_SUFFIX}"
DB_PASSWORD = secrets.token_urlsafe(16)

BENCHMARK_TABLE_SQL = (
    "CREATE TABLE benchmark ("
    "id SERIAL PRIMARY KEY, "
    "version TEXT NOT NULL, "
    "phase TEXT NOT NULL, "
    "duration_ms DOUBLE PRECISION NOT NULL, "
    "recorded_at TIMESTAMPTZ NOT NULL DEFAULT now())"
)


def http_get_health(http_conn: http_client.HTTPConnection) -> int:
    http_conn.request("GET", "/health")
    response = http_conn.getresponse()
    response.read()
    return response.status


def record_and_read_back(conn, version: str, phase: str, duration_ms: float) -> float:
    with conn.cursor() as cur:
        cur.execute(
            "INSERT INTO benchmark (version, phase, duration_ms) VALUES (%s, %s, %s)",
            (version, phase, duration_ms),
        )
        conn.commit()
        cur.execute(
            "SELECT duration_ms FROM benchmark WHERE version = %s AND phase = %s "
            "ORDER BY id DESC LIMIT 1",
            (version, phase),
        )
        row = cur.fetchone()
    return float(row[0])


class UnmanagedPostgresVerifyPlaybook(UnmanagedPlaybook):
    def __init__(self, managed: ManagedPostgresPlaybook):
        self._managed = managed

    def identifier(self) -> str:
        return "bench-postgres-unmanaged-verify"

    def run(self, arena) -> ActivePlaybook:
        active = self._managed.run(arena)
        active.verify("SELECT 1", 1)
        return active


class UnmanagedHttpVerifyPlaybook(UnmanagedPlaybook):
    def __init__(self, http_conn: http_client.HTTPConnection):
        self._http_conn = http_conn

    def identifier(self) -> str:
        return "bench-http-unmanaged-verify"

    def run(self, arena) -> ActivePlaybook:
        assert http_get_health(self._http_conn) == 200
        return ActivePlaybook(None, 0)


def _percentile(values: list[float], pct: float) -> float:
    ordered = sorted(values)
    idx = min(len(ordered) - 1, int(round(pct * (len(ordered) - 1))))
    return ordered[idx]


async def run_iteration(
    n: int, arena, managed_postgres: ManagedPostgresPlaybook, conn, http_conn: http_client.HTTPConnection
) -> float:
    iter_start = asyncio.get_event_loop().time()

    status = await asyncio.to_thread(http_get_health, http_conn)
    assert status == 200
    http_ms = (asyncio.get_event_loop().time() - iter_start) * 1000

    read_back_ms = await asyncio.to_thread(record_and_read_back, conn, VERSION, f"iter-{n}", http_ms)
    assert math.isclose(read_back_ms, http_ms, rel_tol=1e-9)

    active_pg = managed_postgres.run(arena)
    active_pg.verify("SELECT 1", 1)

    return (asyncio.get_event_loop().time() - iter_start) * 1000


async def main() -> None:
    postgres = (
        PostgresDependencyBuilder("bench-postgres")
        .with_port(15432)
        .with_database_name(DB_NAME)
        .with_database_username(DB_USER)
        .with_database_password(DB_PASSWORD)
        .with_startup_sql_scripts([BENCHMARK_TABLE_SQL])
        .build()
    )
    http = HttpDependencyBuilder("bench-http").with_port(18080).build()

    managed_postgres = ManagedPostgresPlaybook(
        identifier="bench-postgres-managed",
        dependency_identifier=postgres.identifier,
    )
    managed_http = ManagedHttpPlaybook.from_builder(
        "bench-http-managed",
        http.identifier,
        lambda b: b.get("/health").will_return(status=200),
    )

    match = (
        MatchBuilder("bench-match")
        .add_dependency(postgres)
        .add_dependency(http)
        .register_playbook(managed_postgres, exec_on_dependency_start=True)
        .register_playbook(managed_http, exec_on_dependency_start=True)
        .build()
    )
    closed = ClosedArena("bench-arena", [match], log_level=ArenaLogLevel.ERROR)

    loop = asyncio.get_event_loop()
    e2e_start = loop.time()

    open_start = loop.time()
    arena = await closed.open()
    open_ms = (loop.time() - open_start) * 1000

    try:
        http_conn = http_client.HTTPConnection("127.0.0.1", 18080)
        conn = psycopg.connect(
            host="127.0.0.1", port=15432, dbname=DB_NAME, user=DB_USER, password=DB_PASSWORD
        )
        try:
            UnmanagedPostgresVerifyPlaybook(managed_postgres).run(arena)
            UnmanagedHttpVerifyPlaybook(http_conn).run(arena)

            iteration_ms = [
                await run_iteration(n, arena, managed_postgres, conn, http_conn)
                for n in range(ITERATIONS)
            ]
        finally:
            conn.close()
            http_conn.close()
    finally:
        close_start = loop.time()
        await arena.close()
        close_ms = (loop.time() - close_start) * 1000

    e2e_ms = (loop.time() - e2e_start) * 1000

    print(
        f"BENCH_RESULT,{open_ms:.3f},{min(iteration_ms):.3f},"
        f"{statistics.median(iteration_ms):.3f},{_percentile(iteration_ms, 0.95):.3f},"
        f"{max(iteration_ms):.3f},{close_ms:.3f},{e2e_ms:.3f}",
        flush=True,
    )


asyncio.run(main())
"""


def main() -> int:
    parser = argparse.ArgumentParser(
        description=(
            "Benchmark a published arena-pytest release: open one arena "
            "(Postgres + HTTP dependency, each with a managed playbook), run N "
            "interact iterations against it (managed playbooks, a real Postgres "
            "read/write round trip), close it, and report timing."
        )
    )
    parser.add_argument("--version", required=True, help="arena-pytest version to install, e.g. 6.1.0")
    parser.add_argument(
        "--pre-release", action="store_true",
        help="install from TestPyPI instead of PyPI (pre-release builds are only published there)",
    )
    parser.add_argument("--iterations", type=int, default=10)
    args = parser.parse_args()

    with tempfile.TemporaryDirectory(prefix="arena-pypi-bench-") as workdir_str:
        workdir = Path(workdir_str)
        venv_dir = workdir / "venv"
        venv.EnvBuilder(with_pip=True, clear=True, symlinks=sys.platform != "win32").create(venv_dir)
        venv_python = venv_dir / ("Scripts/python.exe" if sys.platform == "win32" else "bin/python")

        pip_index_args = (
            ["--index-url", "https://test.pypi.org/simple/", "--extra-index-url", "https://pypi.org/simple/"]
            if args.pre_release else []
        )
        subprocess.run(
            [
                str(venv_python), "-m", "pip", "install", "--quiet",
                *pip_index_args,
                f"arena-pytest=={args.version}", "psycopg[binary]",
            ],
            check=True,
        )

        probe_path = workdir / "probe.py"
        probe_path.write_text(_PROBE_SCRIPT)

        result = subprocess.run(
            [str(venv_python), str(probe_path), args.version, str(args.iterations)],
            capture_output=True,
            text=True,
        )
        sys.stderr.write(result.stderr)
        if result.returncode != 0:
            print(f"bench FAILED: probe exited {result.returncode}", file=sys.stderr)
            return 1

        line = next((l for l in result.stdout.splitlines() if l.startswith("BENCH_RESULT,")), None)
        if line is None:
            print("bench FAILED: no BENCH_RESULT line in probe output", file=sys.stderr)
            return 1

    _, open_ms, min_ms, median_ms, p95_ms, max_ms, close_ms, e2e_ms = line.split(",")
    print(
        f"version={args.version} open_ms={float(open_ms):.2f} iterations={args.iterations} "
        f"interact_min_ms={float(min_ms):.2f} interact_ms={float(median_ms):.2f} "
        f"interact_p95_ms={float(p95_ms):.2f} interact_max_ms={float(max_ms):.2f} "
        f"close_ms={float(close_ms):.2f} e2e_ms={float(e2e_ms):.2f}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
