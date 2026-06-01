package arena.examples.readings.playbooks;

import arena.junit.playbook.ManagedMssqlPlaybook;

public final class ResetValidationDbPlaybook extends ManagedMssqlPlaybook {
  public ResetValidationDbPlaybook(String dependencyIdentifier) {
    super(ReadingsPlaybookConfig.RESET_VALIDATION_DB, dependencyIdentifier);
  }
}
