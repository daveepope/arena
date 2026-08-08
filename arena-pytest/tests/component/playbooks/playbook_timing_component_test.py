from __future__ import annotations

import os
import sys

_TESTS_DIR = os.path.dirname(os.path.abspath(__file__))
if _TESTS_DIR not in sys.path:
    sys.path.insert(0, _TESTS_DIR)

import pytest

from arena_pytest import playbook

from probe_playbooks import CALL_ORDER, ResetProbePlaybook, SeedProbePlaybook


class TestUnmanagedAndManagedPlaybooksStackedOnSameTest:
    @playbook(SeedProbePlaybook)
    @playbook(ResetProbePlaybook)
    def test_1_unmanaged_ran_before_test_body_managed_not_yet_run(self):
        assert list(CALL_ORDER) == ["unmanaged"]

    def test_2_managed_ran_after_step_1_test_body(self):
        assert list(CALL_ORDER) == ["unmanaged", "managed"]


if __name__ == "__main__":
    sys.exit(pytest.main([os.path.dirname(os.path.abspath(__file__)), "-v", "-s"]))
