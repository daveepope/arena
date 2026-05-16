from __future__ import annotations


class HttpReadinessCheck:
    @classmethod
    def create(cls) -> HttpReadinessCheck:
        return cls()


ReadinessCheck = HttpReadinessCheck
