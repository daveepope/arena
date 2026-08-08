package arena.examples.playbooks;

import arena.junit.playbook.ManagedMssqlPlaybook;

public final class ResetValidationDbPlaybook extends ManagedMssqlPlaybook {
  private static final String IDENTIFIER = "example-api-validation-db-scoped";

  public ResetValidationDbPlaybook(String dependencyIdentifier) {
    super(IDENTIFIER, dependencyIdentifier);
  }
}
