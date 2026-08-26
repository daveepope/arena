package arena.examples.playbooks;

import arena.junit.playbook.ManagedLocalstackPlaybook;

public final class EventsPurgePlaybook extends ManagedLocalstackPlaybook {
  private static final String IDENTIFIER = "example-api-events-purge";

  public EventsPurgePlaybook(String dependencyIdentifier) {
    super(IDENTIFIER, dependencyIdentifier);
  }
}
