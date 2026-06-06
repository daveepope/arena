from __future__ import annotations

import requests


class ApiClient:
    def __init__(self, base_url: str, session: requests.Session):
        self._base_url = base_url.rstrip("/")
        self._session = session

    def create_reading(
        self,
        user_name: str,
        value: int,
        comment: str | None,
    ) -> int:
        r = self._session.post(
            f"{self._base_url}/readings",
            json={"user_name": user_name, "value": value, "comment": comment},
            timeout=10,
        )
        r.raise_for_status()
        return int(r.json()["id"])

    def get_readings(self) -> list:
        r = self._session.get(f"{self._base_url}/readings", timeout=10)
        r.raise_for_status()
        return r.json()

    def post_reading_raw(
        self,
        user_name: str,
        value: int,
        comment: str | None,
    ) -> requests.Response:
        return self._session.post(
            f"{self._base_url}/readings",
            json={"user_name": user_name, "value": value, "comment": comment},
            timeout=10,
        )
