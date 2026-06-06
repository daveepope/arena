from arena_pytest import (
    HttpPlaybookBuilder,
    ManagedHttpPlaybook,
    ManagedLocalstackPlaybook,
    ManagedMssqlPlaybook,
    ok_json,
    server_error,
    status,
)

from arena_config import (
    CALIBRATION_VALIDATE_PATH,
    PLAYBOOK_CALIBRATION_API_ERROR_PATH,
    PLAYBOOK_CALIBRATION_API_HAPPY_PATH,
    PLAYBOOK_CALIBRATION_API_FLAKY_PATH,
    PLAYBOOK_EVENTS_PURGE,
    PLAYBOOK_VALIDATION_DB_SCOPED,
)


class CalibrationApiHappyPathPlaybook(ManagedHttpPlaybook):
    def __init__(self, dependency_identifier: str):
        super().__init__(
            identifier=PLAYBOOK_CALIBRATION_API_HAPPY_PATH,
            dependency_identifier=dependency_identifier,
            builder=(
                HttpPlaybookBuilder(dependency_identifier)
                .post(CALIBRATION_VALIDATE_PATH)
                .will_return(ok_json({"valid": True}))
                .expect_called_at_least(1)
            ),
        )


class CalibrationApiErrorPathPlaybook(ManagedHttpPlaybook):
    def __init__(self, dependency_identifier: str):
        super().__init__(
            identifier=PLAYBOOK_CALIBRATION_API_ERROR_PATH,
            dependency_identifier=dependency_identifier,
            builder=(
                HttpPlaybookBuilder(dependency_identifier)
                .post(CALIBRATION_VALIDATE_PATH)
                .will_return(server_error())
            ),
        )


class CalibrationApiFlakyPlaybook(ManagedHttpPlaybook):
    def __init__(self, dependency_identifier: str):
        super().__init__(
            identifier=PLAYBOOK_CALIBRATION_API_FLAKY_PATH,
            dependency_identifier=dependency_identifier,
            builder=(
                HttpPlaybookBuilder(dependency_identifier)
                .post(CALIBRATION_VALIDATE_PATH)
                .will_return(server_error())
                .then_return(status(503))
                .then_return(ok_json({"valid": True}))
            ),
        )


class ResetValidationDbPlaybook(ManagedMssqlPlaybook):
    def __init__(self, dependency_identifier: str):
        super().__init__(
            identifier=PLAYBOOK_VALIDATION_DB_SCOPED,
            dependency_identifier=dependency_identifier,
        )


class EventsPurgePlaybook(ManagedLocalstackPlaybook):
    def __init__(self, dependency_identifier: str):
        super().__init__(
            identifier=PLAYBOOK_EVENTS_PURGE,
            dependency_identifier=dependency_identifier,
        )
