import os
import socket
import uuid


def ephemeral_tcp_port() -> int:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]


def run_suffix() -> str:
    return f"{os.getpid():x}-{uuid.uuid4().hex[:8]}"


class EphemeralTestRuntime:
    def __init__(self) -> None:
        self.run_suffix = run_suffix()
        self.exec_web_app_port = ephemeral_tcp_port()
        self.docker_web_host_port = ephemeral_tcp_port()
        self.kafka_port = ephemeral_tcp_port()
        self.calibration_host_port = ephemeral_tcp_port()
        self.postgres_port = ephemeral_tcp_port()
        self.mssql_port = ephemeral_tcp_port()
        self.oauth_port = ephemeral_tcp_port()
        self.localstack_host_port = ephemeral_tcp_port()
        self.oauth_issuer = f"https://127.0.0.1:{self.oauth_port}"
        os.environ["ARENA_PYTEST_OAUTH_ISSUER"] = self.oauth_issuer

    def network_name(self, base: str) -> str:
        return f"{base}-{self.run_suffix}"

    def container_name(self, base: str) -> str:
        return f"{base}-{self.run_suffix}"


RUNTIME = EphemeralTestRuntime()
