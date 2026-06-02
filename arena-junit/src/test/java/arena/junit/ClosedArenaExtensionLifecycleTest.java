package arena.junit;

import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertSame;
import static org.junit.jupiter.api.Assertions.assertTrue;

import arena.examples.readings.testruntime.ReadingsEphemeralTestRuntime;
import arena.junit.dep.HttpDependency;
import arena.junit.dep.HttpDependencyBuilder;
import arena.junit.match.Match;
import arena.junit.match.MatchBuilder;
import arena.junit.oauth.OauthDependency;
import arena.junit.oauth.OauthDependencyBuilder;
import arena.junit.oauth.OauthLoopbackTls;
import java.util.List;
import org.junit.jupiter.api.MethodOrderer;
import org.junit.jupiter.api.Order;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.TestMethodOrder;
import org.junit.jupiter.api.extension.RegisterExtension;

@TestMethodOrder(MethodOrderer.OrderAnnotation.class)
final class ClosedArenaExtensionLifecycleTest {

  private static final ReadingsEphemeralTestRuntime RT = ReadingsEphemeralTestRuntime.get();

  static final class OauthLifecycleFixture extends ClosedArenaExtension {
    static int buildCount;

    @Override
    protected ClosedArena buildClosedArena() throws Exception {
      buildCount++;
      OauthLoopbackTls.PemPair pem = OauthLoopbackTls.oauthLoopbackTlsPemPair();
      OauthDependency oauth =
          new OauthDependencyBuilder("lifecycle-oauth")
              .withPort(RT.oauthPort)
              .withListenIp("0.0.0.0")
              .withServerTlsPem(pem.certificatePem(), pem.privateKeyPem())
              .withMetadataBaseUrl(RT.oauthIssuer)
              .build();
      Match match = new MatchBuilder("lifecycle-match").addDependency(oauth).build();
      return new ClosedArena("lifecycle-arena", List.of(match));
    }
  }

  @RegisterExtension
  static final OauthLifecycleFixture lifecycleArena = new OauthLifecycleFixture();

  @Test
  @Order(1)
  void openArena_firstTest_returnsOpenHandle() {
    OpenArena arena = lifecycleArena.openArena();
    assertNotNull(arena);
    assertTrue(arena.handle() != null);
  }

  @Test
  @Order(2)
  void openArena_secondTest_returnsSameInstance() {
    OpenArena first = lifecycleArena.openArena();
    OpenArena again = lifecycleArena.openArena();
    assertSame(first, again);
    assertTrue(OauthLifecycleFixture.buildCount >= 1);
  }
}
