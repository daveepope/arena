package arena.junit.oauth;
import arena.junit.OpenArena;
import arena.junit.ffi.ArenaBindings;
import arena.junit.match.ArenaRunnableDependency;

import com.fasterxml.jackson.databind.node.ObjectNode;

public final class OauthDependency implements ArenaRunnableDependency {
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

  public String signClaims(OpenArena arena, int issuerIndex, String claimsJson) {
    return ArenaBindings.oauthSignClaims(arena.handle(), identifier(), issuerIndex, claimsJson);
  }
}
