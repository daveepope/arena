from arena_pytest import HttpPlaybookBuilder, ManagedHttpPlaybook, server_error

from readings_arena_config import (
    CALIBRATION_VALIDATE_PATH,
)

PLAYBOOK_CALIBRATION_OUTAGE_VERIFY_PROBE = (
    "arena-pytest-calibration-outage-verify-probe"
)


class CalibrationOutageVerifyProbePlaybook(ManagedHttpPlaybook):
    def __init__(self, dependency_identifier: str):
        mappings = (
            HttpPlaybookBuilder(dependency_identifier)
            .post(CALIBRATION_VALIDATE_PATH)
            .will_return(server_error())
            .into_playbook()
            .mappings_for_ffi()
        )
        super().__init__(
            identifier=PLAYBOOK_CALIBRATION_OUTAGE_VERIFY_PROBE,
            dependency_identifier=dependency_identifier,
            mappings=mappings,
        )
