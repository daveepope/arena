package arena.junit.oauth;
import java.net.URI;
import java.util.Locale;

public final class OauthIssuerHosts {
  private OauthIssuerHosts() {}

  public static boolean oauthIssuerHostIsNonLoopback() {
    try {
      String host = URI.create(OauthDependencyBuilder.OAUTH_ISSUER).getHost();
      if (host == null || host.isEmpty()) {
        return false;
      }
      String h = host.toLowerCase(Locale.ROOT);
      return !h.equals("127.0.0.1") && !h.equals("localhost") && !h.equals("::1");
    } catch (Exception e) {
      return false;
    }
  }
}
