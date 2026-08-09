package arena.junit.dep.temporal;
import arena.junit.match.ArenaRunnableDependency;

import com.fasterxml.jackson.databind.node.ObjectNode;

public final class TemporalDependency implements ArenaRunnableDependency {
  private final ObjectNode config;

  TemporalDependency(ObjectNode config) {
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
