package arena.junit.dep.oracledb;
import arena.junit.match.ArenaRunnableDependency;

import com.fasterxml.jackson.databind.node.ObjectNode;

public final class OracleDependency implements ArenaRunnableDependency {
  private final ObjectNode config;

  OracleDependency(ObjectNode config) {
    this.config = config;
  }

  public String identifier() {
    return config.get("identifier").asText();
  }

  @Override
  public ObjectNode forFfi() {
    return config;
  }
}
