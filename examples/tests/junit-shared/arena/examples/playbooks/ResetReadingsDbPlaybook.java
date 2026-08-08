package arena.examples.playbooks;

import arena.junit.playbook.ManagedPostgresPlaybook;

public final class ResetReadingsDbPlaybook extends ManagedPostgresPlaybook {
  private static final String IDENTIFIER = "example-api-readings-db-scoped";

  public ResetReadingsDbPlaybook(String dependencyIdentifier) {
    super(IDENTIFIER, dependencyIdentifier);
  }
}
