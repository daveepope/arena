from arena_pytest import ManagedHttpPlaybook

from readings_arena_config import (
    CALIBRATION_VALIDATE_PATH,
)

PLAYBOOK_CALIBRATION_OUTAGE_VERIFY_PROBE = (
    "arena-pytest-calibration-outage-verify-probe"
)


class CalibrationOutageVerifyProbePlaybook(ManagedHttpPlaybook):
    def __init__(self, dependency_identifier: str):
        super().__init__(
            identifier=PLAYBOOK_CALIBRATION_OUTAGE_VERIFY_PROBE,
            dependency_identifier=dependency_identifier,
            mappings=[
                {
                    "method": "POST",
                    "url_path": CALIBRATION_VALIDATE_PATH,
                    "status": 500,
                }
            ],
        )
