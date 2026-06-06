package arena.examples.playbooks;

import arena.junit.playbook.ManagedLocalstackPlaybook;

public final class EventsPurgePlaybook extends ManagedLocalstackPlaybook {
  public EventsPurgePlaybook(String dependencyIdentifier) {
    super(PlaybookConfig.EVENTS_PURGE, dependencyIdentifier);
  }
}
