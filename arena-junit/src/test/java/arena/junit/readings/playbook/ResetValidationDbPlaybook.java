package arena.junit.readings.playbook;

import arena.junit.playbook.ManagedMssqlPlaybook;
import arena.junit.readings.fixture.ReadingsArenaConfig;

public final class ResetValidationDbPlaybook extends ManagedMssqlPlaybook {
  public ResetValidationDbPlaybook(String dependencyIdentifier) {
    super(ReadingsArenaConfig.PLAYBOOK_VALIDATION_DB_SCOPED, dependencyIdentifier);
  }
}
