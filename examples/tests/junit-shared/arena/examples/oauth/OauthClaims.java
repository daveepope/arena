package arena.examples.oauth;

import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.node.ObjectNode;

public final class OauthClaims {

  private OauthClaims() {}

  public static String withScope(ObjectMapper mapper, String issuer, String scope)
      throws Exception {
    long now = System.currentTimeMillis() / 1000L;
    ObjectNode claims =
        mapper
            .createObjectNode()
            .put("iss", issuer)
            .put("sub", "arena-examples")
            .put("scope", scope)
            .put("iat", now)
            .put("exp", now + 300);
    return mapper.writeValueAsString(claims);
  }
}
