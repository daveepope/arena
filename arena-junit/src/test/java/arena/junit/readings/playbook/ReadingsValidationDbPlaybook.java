package arena.junit.readings.playbook;

import arena.junit.playbook.ManagedMssqlPlaybook;
import arena.junit.readings.fixture.ReadingsArenaConfig;

public final class ReadingsValidationDbPlaybook extends ManagedMssqlPlaybook {
  public ReadingsValidationDbPlaybook(String dependencyIdentifier) {
    super(ReadingsArenaConfig.PLAYBOOK_VALIDATION_DB_SCOPED, dependencyIdentifier);
  }
}
