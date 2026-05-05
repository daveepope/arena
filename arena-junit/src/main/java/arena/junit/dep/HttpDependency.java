package arena.junit.dep;
import arena.junit.match.ArenaMatchPiece;

import com.fasterxml.jackson.databind.node.ObjectNode;

public final class HttpDependency implements ArenaMatchPiece {
  private final ObjectNode config;

  HttpDependency(ObjectNode config) {
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
