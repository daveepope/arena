import os
import uuid

from arena_pytest.host import PortSearchStrategy, find_available_port

PORT_SLOT_COUNT = 13
EPHEMERAL_PORT_RANGE_START = 20000
EPHEMERAL_PORT_RANGE_END = 20300


def _allocate_distinct_tcp_ports(count: int) -> list[int]:
    return [
        find_available_port(
            EPHEMERAL_PORT_RANGE_START, EPHEMERAL_PORT_RANGE_END, PortSearchStrategy.RANDOM
        )
        for _ in range(count)
    ]


def ephemeral_tcp_port() -> int:
    return _allocate_distinct_tcp_ports(1)[0]


def run_suffix() -> str:
    return uuid.uuid4().hex


class EphemeralTestRuntime:
    def __init__(self) -> None:
        self.run_suffix = run_suffix()
        (
            self.exec_web_app_port,
            self.docker_web_host_port,
            self.kafka_port,
            self.calibration_host_port,
            self.postgres_port,
            self.mssql_port,
            self.oracle_port,
            self.oauth_port,
            self.localstack_host_port,
            self.temporal_grpc_port,
            self.temporal_ui_port,
            self.smtp_port,
            self.smtp_ui_port,
        ) = _allocate_distinct_tcp_ports(PORT_SLOT_COUNT)
        self.oauth_issuer = f"https://127.0.0.1:{self.oauth_port}"
        os.environ["ARENA_PYTEST_OAUTH_ISSUER"] = self.oauth_issuer

    def namespaced(self, base: str) -> str:
        return f"{base}-{self.run_suffix}"

    def network_name(self, base: str) -> str:
        return self.namespaced(base)

    def container_name(self, base: str) -> str:
        return self.namespaced(base)


RUNTIME = EphemeralTestRuntime()
