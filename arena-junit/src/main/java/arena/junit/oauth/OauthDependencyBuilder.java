package arena.junit.oauth;
import arena.junit.support.ArenaIdentifiers;
import arena.junit.support.ArenaJson;

import com.fasterxml.jackson.databind.node.ObjectNode;

public final class OauthDependencyBuilder {
  public static final int DEFAULT_OAUTH_PORT = 9444;

  public static final String OAUTH_ISSUER = issuerFromEnv();

  private static String issuerFromEnv() {
    String v = System.getenv("ARENA_PYTEST_OAUTH_ISSUER");
    if (v != null) {
      v = v.strip();
      if (!v.isEmpty()) {
        return v.replaceAll("/+$", "");
      }
    }
    return "https://127.0.0.1:" + DEFAULT_OAUTH_PORT;
  }

  private final ObjectNode config =
      ArenaJson.object()
          .put("type", "oauth")
          .put("identifier", ArenaIdentifiers.build("arena-oauth", ""))
          .put("port", DEFAULT_OAUTH_PORT);

  public OauthDependencyBuilder(String name) {
    config.put("identifier", ArenaIdentifiers.build("arena-oauth", name));
  }

  public OauthDependencyBuilder withPort(int port) {
    config.put("port", port);
    return this;
  }

  public OauthDependencyBuilder withListenIp(String ip) {
    config.put("listen_ip", ip);
    return this;
  }

  public OauthDependencyBuilder withServerTlsPem(String certPem, String keyPem) {
    config.put("server_tls_certificate_pem", certPem);
    config.put("server_tls_private_key_pem", keyPem);
    return this;
  }

  public OauthDependencyBuilder withMetadataBaseUrl(String url) {
    config.put("metadata_base_url", url.replaceAll("/+$", ""));
    return this;
  }

  public OauthDependency build() {
    ObjectNode cfg = config.deepCopy();
    if (!cfg.has("metadata_base_url") || cfg.get("metadata_base_url").asText("").isBlank()) {
      cfg.put("metadata_base_url", OAUTH_ISSUER);
    }
    return new OauthDependency(cfg);
  }
}
