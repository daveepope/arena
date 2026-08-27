package arena.junit.oauth;

import static org.junit.jupiter.api.Assertions.assertEquals;

import arena.examples.testruntime.EphemeralTestRuntime;
import arena.junit.Arena;
import arena.junit.ArenaDependency;
import org.junit.jupiter.api.Test;

@Arena(OauthSignerParameterResolverComponentTest.Fixture.class)
@ArenaOauthSigner
final class OauthSignerParameterResolverComponentTest {

  static class Fixture {
    @ArenaDependency
    static final OauthDependency OAUTH =
        new OauthDependencyBuilder("oauth-signer-resolver-test")
            .withPort(EphemeralTestRuntime.ephemeralTcpPort())
            .withHttp()
            .build();
  }

  @Test
  void signerParameterIsInjectedAndProducesVerifiableToken(OauthSigner signer) {
    String jwt = signer.sign("{\"sub\":\"test-user\",\"iat\":0,\"exp\":9999999999}");
    assertEquals(3, jwt.split("\\.").length);
  }
}
