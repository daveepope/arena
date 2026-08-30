import os
import sys

import pytest

from ephemeral_test_runtime import (
    EPHEMERAL_PORT_RANGE_END,
    EPHEMERAL_PORT_RANGE_START,
    PORT_SLOT_COUNT,
    TARGET_PORT_RANGES,
    EphemeralTestRuntime,
    ephemeral_tcp_port,
    port_range_for_target,
)


def test_ephemeral_tcp_port_bind_returns_nonzero_port():
    port = ephemeral_tcp_port()
    assert 1024 <= port <= 65535


def test_ephemeral_runtime_ports_are_pairwise_distinct():
    rt = EphemeralTestRuntime()
    ports = (
        rt.exec_web_app_port,
        rt.docker_web_host_port,
        rt.kafka_port,
        rt.calibration_host_port,
        rt.postgres_port,
        rt.mssql_port,
        rt.oracle_port,
        rt.oauth_port,
        rt.localstack_host_port,
        rt.temporal_grpc_port,
        rt.temporal_ui_port,
        rt.smtp_port,
        rt.smtp_ui_port,
    )
    assert len(ports) == PORT_SLOT_COUNT
    assert len(ports) == len(set(ports))


@pytest.mark.parametrize("target", sorted(TARGET_PORT_RANGES))
def test_port_range_for_target_known_target_returns_slot_with_room(target):
    start, end = port_range_for_target(target)

    assert (start, end) == TARGET_PORT_RANGES[target]
    assert end - start >= PORT_SLOT_COUNT


@pytest.mark.parametrize("target", [None, "", "//examples:not_a_real_target"])
def test_port_range_for_target_unknown_target_returns_full_range(target):
    assert port_range_for_target(target) == (
        EPHEMERAL_PORT_RANGE_START,
        EPHEMERAL_PORT_RANGE_END,
    )


def test_target_port_ranges_are_pairwise_disjoint():
    claimed = set()
    for start, end in TARGET_PORT_RANGES.values():
        assert start >= EPHEMERAL_PORT_RANGE_START
        assert end <= EPHEMERAL_PORT_RANGE_END
        slots = set(range(start, end))
        assert not slots & claimed
        claimed |= slots


def test_ephemeral_runtime_namespaced_matches_network_and_container_name():
    rt = EphemeralTestRuntime()
    base = "example-api-postgres"
    assert rt.network_name(base) == rt.container_name(base)
    assert rt.namespaced(base) == f"{base}-{rt.run_suffix}"


if __name__ == "__main__":
    sys.exit(pytest.main([os.path.dirname(os.path.abspath(__file__)), "-v", "-s"]))
