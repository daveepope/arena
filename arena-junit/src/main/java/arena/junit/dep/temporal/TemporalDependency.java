package arena.junit.dep.temporal;
import arena.junit.match.ArenaMatchPiece;

import com.fasterxml.jackson.databind.node.ObjectNode;

public final class TemporalDependency implements ArenaMatchPiece {
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
