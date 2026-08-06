from __future__ import annotations


class HttpReadinessCheck:
    @classmethod
    def create(cls) -> HttpReadinessCheck:
        return cls()


class TcpReadinessCheck:
    @classmethod
    def create(cls) -> TcpReadinessCheck:
        return cls()


ReadinessCheck = HttpReadinessCheck
