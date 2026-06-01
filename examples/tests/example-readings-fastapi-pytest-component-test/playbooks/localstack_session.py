from arena_pytest import ManagedLocalstackPlaybook

from readings_arena_config import PLAYBOOK_LOCALSTACK_SESSION


class LocalstackSessionPlaybook(ManagedLocalstackPlaybook):
    def __init__(self, dependency_identifier: str):
        super().__init__(
            identifier=PLAYBOOK_LOCALSTACK_SESSION,
            dependency_identifier=dependency_identifier,
        )
