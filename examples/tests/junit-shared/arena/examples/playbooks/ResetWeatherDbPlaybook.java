package arena.examples.playbooks;

import arena.junit.playbook.oracledb.ManagedOraclePlaybook;

public final class ResetWeatherDbPlaybook extends ManagedOraclePlaybook {
  private static final String IDENTIFIER = "example-api-weather-db-scoped";

  public ResetWeatherDbPlaybook(String dependencyIdentifier) {
    super(IDENTIFIER, dependencyIdentifier);
  }
}
