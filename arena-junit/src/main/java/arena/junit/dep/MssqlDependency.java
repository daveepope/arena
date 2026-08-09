package arena.junit.dep;
import arena.junit.match.ArenaRunnableDependency;

import com.fasterxml.jackson.databind.node.ObjectNode;

public final class MssqlDependency implements ArenaRunnableDependency {
  private final ObjectNode config;

  MssqlDependency(ObjectNode config) {
    this.config = config;
  }

  @Override
  public ObjectNode forFfi() {
    return config;
  }

  public String identifier() {
    return config.get("identifier").asText();
  }
}
