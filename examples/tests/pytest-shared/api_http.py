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
        device_id: int | None = None,
    ) -> int:
        r = self.post_reading_raw(user_name, value, comment, device_id)
        r.raise_for_status()
        return int(r.json()["id"])

    def get_readings(self) -> list:
        r = self._get("/readings")
        r.raise_for_status()
        return r.json()

    def post_reading_raw(
        self,
        user_name: str,
        value: int,
        comment: str | None,
        device_id: int | None = None,
    ) -> requests.Response:
        body = {"user_name": user_name, "value": value, "comment": comment}
        if device_id is not None:
            body["device_id"] = device_id
        return self._post("/readings", body)

    def create_device(self, name: str) -> int:
        r = self._post("/devices", {"name": name})
        r.raise_for_status()
        return int(r.json()["id"])

    def get_device_state_raw(self, device_id: int) -> requests.Response:
        return self._get(f"/devices/{device_id}/state")

    def get_device_state(self, device_id: int) -> str:
        r = self.get_device_state_raw(device_id)
        r.raise_for_status()
        return str(r.json()["state"])

    def set_device_state(self, device_id: int, target: str) -> None:
        r = self._post(f"/devices/{device_id}/state", {"target": target})
        r.raise_for_status()

    def stop_device(self, device_id: int) -> None:
        r = self._delete(f"/devices/{device_id}")
        r.raise_for_status()

    def _get(self, path: str) -> requests.Response:
        return self._session.get(f"{self._base_url}{path}", timeout=10)

    def _post(self, path: str, body: dict) -> requests.Response:
        return self._session.post(f"{self._base_url}{path}", json=body, timeout=10)

    def _delete(self, path: str) -> requests.Response:
        return self._session.delete(f"{self._base_url}{path}", timeout=10)
