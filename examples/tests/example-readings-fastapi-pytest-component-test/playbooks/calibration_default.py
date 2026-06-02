from arena_pytest import HttpMappingExpect, ManagedHttpPlaybook

from readings_arena_config import (
    CALIBRATION_VALIDATE_PATH,
    PLAYBOOK_CALIBRATION_DEFAULT,
)


class CalibrationDefaultPlaybook(ManagedHttpPlaybook):
    def __init__(self, dependency_identifier: str):
        super().__init__(
            identifier=PLAYBOOK_CALIBRATION_DEFAULT,
            dependency_identifier=dependency_identifier,
            mappings=[
                {
                    "method": "POST",
                    "url_path": CALIBRATION_VALIDATE_PATH,
                    "status": 200,
                    "json_body": {"valid": True},
                    "expect": HttpMappingExpect.called_at_least(1),
                }
            ],
        )
