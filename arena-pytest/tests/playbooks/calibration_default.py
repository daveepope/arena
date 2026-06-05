from arena_pytest import HttpPlaybookBuilder, ManagedHttpPlaybook, ok_json

from readings_arena_config import (
    CALIBRATION_VALIDATE_PATH,
    PLAYBOOK_CALIBRATION_DEFAULT,
)


class CalibrationDefaultPlaybook(ManagedHttpPlaybook):
    def __init__(self, dependency_identifier: str):
        super().__init__(
            identifier=PLAYBOOK_CALIBRATION_DEFAULT,
            dependency_identifier=dependency_identifier,
            builder=(
                HttpPlaybookBuilder(dependency_identifier)
                .post(CALIBRATION_VALIDATE_PATH)
                .will_return(ok_json({"valid": True}))
                .expect_called_at_least(1)
            ),
        )
