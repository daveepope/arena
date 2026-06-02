from arena_pytest import ManagedLocalstackPlaybook

MANAGED_LOCALSTACK_PLAYBOOK_ID = "localstack-session-purge"


class LocalstackSessionPurgePlaybook(ManagedLocalstackPlaybook):
    def __init__(self, dependency_identifier: str):
        super().__init__(
            identifier=MANAGED_LOCALSTACK_PLAYBOOK_ID,
            dependency_identifier=dependency_identifier,
        )
