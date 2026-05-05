package dev.arena.junit.oauth;
import dev.arena.junit.match.ArenaMatchPiece;

import com.fasterxml.jackson.databind.node.ObjectNode;

public final class OauthDependency implements ArenaMatchPiece {
  private final ObjectNode config;

  OauthDependency(ObjectNode config) {
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
