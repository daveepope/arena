package arena.examples.playbooks;

import arena.junit.playbook.ManagedMssqlPlaybook;

public final class ResetValidationDbPlaybook extends ManagedMssqlPlaybook {
  public ResetValidationDbPlaybook(String dependencyIdentifier) {
    super(PlaybookConfig.RESET_VALIDATION_DB, dependencyIdentifier);
  }
}
