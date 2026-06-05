from arena_pytest import HttpPlaybookBuilder, ManagedHttpPlaybook, server_error

from readings_arena_config import (
    CALIBRATION_VALIDATE_PATH,
    PLAYBOOK_CALIBRATION_OUTAGE_MANAGED,
)


class CalibrationOutagePlaybook(ManagedHttpPlaybook):
    def __init__(self, dependency_identifier: str):
        mappings = (
            HttpPlaybookBuilder(dependency_identifier)
            .post(CALIBRATION_VALIDATE_PATH)
            .will_return(server_error())
            .expect_called_at_least(1)
            .into_playbook()
            .mappings_for_ffi()
        )
        super().__init__(
            identifier=PLAYBOOK_CALIBRATION_OUTAGE_MANAGED,
            dependency_identifier=dependency_identifier,
            mappings=mappings,
        )
