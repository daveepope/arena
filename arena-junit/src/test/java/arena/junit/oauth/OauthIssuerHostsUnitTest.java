package arena.junit.oauth;

import static org.junit.jupiter.api.Assertions.assertFalse;

import org.junit.jupiter.api.Test;

final class OauthIssuerHostsUnitTest {

  @Test
  void oauthIssuerHostIsNonLoopback_defaultLoopbackIssuer_returnsFalse() {
    assertFalse(OauthIssuerHosts.oauthIssuerHostIsNonLoopback());
  }
}
