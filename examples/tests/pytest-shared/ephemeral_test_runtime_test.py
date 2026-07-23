import os
import sys

import pytest

from ephemeral_test_runtime import (
    PORT_SLOT_COUNT,
    EphemeralTestRuntime,
    ephemeral_tcp_port,
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
        rt.oauth_port,
        rt.localstack_host_port,
        rt.temporal_grpc_port,
        rt.temporal_ui_port,
    )
    assert len(ports) == PORT_SLOT_COUNT
    assert len(ports) == len(set(ports))


def test_ephemeral_runtime_namespaced_matches_network_and_container_name():
    rt = EphemeralTestRuntime()
    base = "example-api-postgres"
    assert rt.network_name(base) == rt.container_name(base)
    assert rt.namespaced(base) == f"{base}-{rt.run_suffix}"


if __name__ == "__main__":
    sys.exit(pytest.main([os.path.dirname(os.path.abspath(__file__)), "-v", "-s"]))
