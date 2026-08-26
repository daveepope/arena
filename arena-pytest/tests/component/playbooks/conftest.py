from __future__ import annotations

import os
import sys

import pytest

_TESTS_DIR = os.path.dirname(os.path.abspath(__file__))
if _TESTS_DIR not in sys.path:
    sys.path.insert(0, _TESTS_DIR)


def _arena_plugin_already_registered_via_entry_point() -> bool:
    try:
        from importlib.metadata import entry_points

        eps = entry_points(group="pytest11")
    except Exception:
        return False
    return any(getattr(ep, "value", "") == "arena_pytest.arena" for ep in eps)


if not _arena_plugin_already_registered_via_entry_point():
    pytest_plugins = ("arena_pytest.arena",)

from arena_pytest import ClosedArena, MatchBuilder, MssqlDependencyBuilder

from probe_playbooks import ResetProbePlaybook, SeedProbePlaybook

MATCH_NAME = "playbook-timing-probe"
DEP_NAME_MSSQL = "playbook-timing-probe-mssql"
MSSQL_PORT = 15433
MSSQL_DB_NAME = "playbookTimingProbeDb"
MSSQL_DB_USER = "sa"
MSSQL_DB_PASS = "yourStrong(!)Password"


@pytest.fixture(scope="session")
def closed_arena() -> ClosedArena:
    mssql = (
        MssqlDependencyBuilder(DEP_NAME_MSSQL)
        .with_port(MSSQL_PORT)
        .with_database_name(MSSQL_DB_NAME)
        .with_database_username(MSSQL_DB_USER)
        .with_database_password(MSSQL_DB_PASS)
        .build()
    )

    a_match = (
        MatchBuilder(MATCH_NAME)
        .add_dependency(mssql)
        .register_playbook(ResetProbePlaybook(mssql.identifier))
        .register_playbook(SeedProbePlaybook())
        .build()
    )

    return ClosedArena(MATCH_NAME, [a_match])
