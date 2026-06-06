from __future__ import annotations

import pytest

from arena_pytest.arena import active_playbooks_for_item
from arena_pytest.playbook import ActiveHttpPlaybook


class _LazyActiveHttpPlaybook:
    def __init__(self, request: pytest.FixtureRequest):
        self._request = request

    def _resolve(self) -> ActiveHttpPlaybook:
        http_actives = [
            a for a in active_playbooks_for_item(self._request.node) if isinstance(a, ActiveHttpPlaybook)
        ]
        if len(http_actives) != 1:
            raise RuntimeError(
                "expected exactly one ActiveHttpPlaybook from stacked @playbook markers"
            )
        return http_actives[0]

    def verify(self, method: str, url_path: str, expected_count: int) -> None:
        self._resolve().verify(method, url_path, expected_count)

    def verify_at_least(self, method: str, url_path: str, minimum_count: int) -> None:
        self._resolve().verify_at_least(method, url_path, minimum_count)


@pytest.fixture
def active_http_playbook(request: pytest.FixtureRequest) -> _LazyActiveHttpPlaybook:
    return _LazyActiveHttpPlaybook(request)
