from arena_pytest import HttpMappingExpect, ManagedHttpPlaybook

from readings_arena_config import (
    CALIBRATION_VALIDATE_PATH,
    PLAYBOOK_CALIBRATION_OUTAGE_MANAGED,
)


class CalibrationOutagePlaybook(ManagedHttpPlaybook):
    def __init__(self, dependency_identifier: str):
        super().__init__(
            identifier=PLAYBOOK_CALIBRATION_OUTAGE_MANAGED,
            dependency_identifier=dependency_identifier,
            mappings=[
                {
                    "method": "POST",
                    "url_path": CALIBRATION_VALIDATE_PATH,
                    "status": 500,
                    "expect": HttpMappingExpect.called_at_least(1),
                }
            ],
        )
