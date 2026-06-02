from arena_pytest import ManagedMssqlPlaybook

from readings_arena_config import PLAYBOOK_VALIDATION_DB_SCOPED


class ResetValidationDbPlaybook(ManagedMssqlPlaybook):
    def __init__(self, dependency_identifier: str):
        super().__init__(
            identifier=PLAYBOOK_VALIDATION_DB_SCOPED,
            dependency_identifier=dependency_identifier,
        )
