from arena_pytest import (
    HttpMappingExpect,
    ManagedHttpPlaybook,
    ManagedLocalstackPlaybook,
    ManagedMssqlPlaybook,
)

CALIBRATION_VALIDATE_PATH = "/api/v1/validate"
LOCALSTACK_SESSION_PLAYBOOK_ID = "readings-api-localstack-session"
CALIBRATION_DEFAULT_PLAYBOOK_ID = "readings-api-calibration-default"
VALIDATION_DB_PLAYBOOK_ID = "readings-api-validation-db-scoped"


class CalibrationDefaultPlaybook(ManagedHttpPlaybook):
    def __init__(self, dependency_identifier: str):
        super().__init__(
            identifier=CALIBRATION_DEFAULT_PLAYBOOK_ID,
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


class ValidationDbPlaybook(ManagedMssqlPlaybook):
    def __init__(self, dependency_identifier: str):
        super().__init__(
            identifier=VALIDATION_DB_PLAYBOOK_ID,
            dependency_identifier=dependency_identifier,
        )


class LocalstackSessionPlaybook(ManagedLocalstackPlaybook):
    def __init__(self, dependency_identifier: str):
        super().__init__(
            identifier=LOCALSTACK_SESSION_PLAYBOOK_ID,
            dependency_identifier=dependency_identifier,
        )
