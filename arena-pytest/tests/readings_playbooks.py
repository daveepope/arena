from arena_pytest import ManagedHttpPlaybook, ManagedMssqlPlaybook, HttpMappingExpect

from readings_arena_config import (
    CALIBRATION_VALIDATE_PATH,
    PLAYBOOK_CALIBRATION_DEFAULT,
    PLAYBOOK_CALIBRATION_OUTAGE_MANAGED,
    PLAYBOOK_VALIDATION_DB_SCOPED,
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


class ValidationDbPlaybook(ManagedMssqlPlaybook):
    def __init__(self, dependency_identifier: str):
        super().__init__(
            identifier=PLAYBOOK_VALIDATION_DB_SCOPED,
            dependency_identifier=dependency_identifier,
        )
