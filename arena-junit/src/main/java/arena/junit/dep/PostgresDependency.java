package arena.junit.dep;
import arena.junit.match.ArenaMatchPiece;

import com.fasterxml.jackson.databind.node.ObjectNode;

public final class PostgresDependency implements ArenaMatchPiece {
  private final ObjectNode config;

  PostgresDependency(ObjectNode config) {
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
