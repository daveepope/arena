package arena.junit.oauth;
import arena.junit.ffi.ArenaBindingError;
import arena.junit.ffi.ArenaBindings;
import arena.junit.support.ArenaJson;

import com.fasterxml.jackson.databind.JsonNode;

public final class OauthLoopbackTls {
  public record PemPair(String certificatePem, String privateKeyPem) {}

  private OauthLoopbackTls() {}

  public static PemPair oauthLoopbackTlsPemPair() {
    try {
      String raw = ArenaBindings.oauthLoopbackTlsPemJson();
      JsonNode n = ArenaJson.MAPPER.readTree(raw);
      JsonNode cert = n.get("certificate_pem");
      JsonNode key = n.get("private_key_pem");
      if (cert == null || key == null || !cert.isTextual() || !key.isTextual()) {
        throw new ArenaBindingError("arena_oauth_loopback_tls_pem_json: missing certificate_pem or private_key_pem");
      }
      return new PemPair(cert.asText(), key.asText());
    } catch (ArenaBindingError e) {
      throw e;
    } catch (Exception e) {
      throw new ArenaBindingError(e.getMessage());
    }
  }
}
