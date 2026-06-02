package arena.examples.readings.playbooks;

import arena.junit.playbook.ManagedLocalstackPlaybook;

public final class LocalstackSessionPlaybook extends ManagedLocalstackPlaybook {
  public LocalstackSessionPlaybook(String dependencyIdentifier) {
    super(ReadingsPlaybookConfig.LOCALSTACK_SESSION, dependencyIdentifier);
  }
}
