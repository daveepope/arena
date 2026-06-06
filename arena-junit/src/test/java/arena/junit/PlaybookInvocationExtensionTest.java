package arena.junit;

import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertThrows;

import arena.examples.testruntime.EphemeralTestRuntime;
import arena.junit.dep.HttpDependency;
import arena.junit.dep.HttpDependencyBuilder;
import arena.junit.match.Match;
import arena.junit.match.MatchBuilder;
import arena.junit.oauth.OauthDependency;
import arena.junit.oauth.OauthDependencyBuilder;
import arena.junit.oauth.OauthLoopbackTls;
import arena.junit.playbook.HttpPlaybookBuilder;
import arena.junit.playbook.HttpResponse;
import arena.junit.playbook.ManagedHttpPlaybook;
import arena.junit.playbook.Playbook;
import java.util.List;
import java.util.Map;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.extension.RegisterExtension;

final class PlaybookInvocationExtensionTest {

  private static final EphemeralTestRuntime RT = EphemeralTestRuntime.get();
  private static final String CALIBRATION_VALIDATE_PATH = "/api/v1/validate";

  static final class HttpPlaybookArenaFixture extends ClosedArenaExtension {
    private String calibrationIdentifier;

    String calibrationIdentifier() {
      return calibrationIdentifier;
    }

    @Override
    protected ClosedArena buildClosedArena() throws Exception {
      OauthLoopbackTls.PemPair pem = OauthLoopbackTls.oauthLoopbackTlsPemPair();
      OauthDependency oauth =
          new OauthDependencyBuilder("playbook-ext-oauth")
              .withPort(RT.oauthPort)
              .withListenIp("0.0.0.0")
              .withServerTlsPem(pem.certificatePem(), pem.privateKeyPem())
              .withMetadataBaseUrl(RT.oauthIssuer)
              .build();
      HttpDependency calibration =
          new HttpDependencyBuilder("playbook-ext-calibration")
              .withPort(RT.calibrationHostPort)
              .build();
      calibrationIdentifier = calibration.identifier();
      SessionDefaultPlaybook sessionDefault =
          new SessionDefaultPlaybook(calibration.identifier());
      ScopedOutagePlaybook scopedOutage =
          new ScopedOutagePlaybook(calibration.identifier());
      ScopedResetPlaybook scopedReset = new ScopedResetPlaybook(calibration.identifier());
      Match match =
          new MatchBuilder("playbook-ext-match")
              .addDependency(oauth)
              .addDependency(calibration)
              .registerPlaybook(sessionDefault, true)
              .registerPlaybook(scopedOutage, false)
              .registerPlaybook(scopedReset, false)
              .build();
      return new ClosedArena("playbook-ext-arena", List.of(match));
    }
  }

  static final class SessionDefaultPlaybook extends ManagedHttpPlaybook {
    SessionDefaultPlaybook(String dependencyIdentifier) {
      super(
          "playbook-ext-session-default",
          dependencyIdentifier,
          new HttpPlaybookBuilder(dependencyIdentifier)
              .post(CALIBRATION_VALIDATE_PATH)
              .willReturn(HttpResponse.okJson(Map.of("valid", true)))
              .expectCalledAtLeast(1));
    }
  }

  static final class ScopedOutagePlaybook extends ManagedHttpPlaybook {
    ScopedOutagePlaybook(String dependencyIdentifier) {
      super(
          "playbook-ext-scoped-outage",
          dependencyIdentifier,
          new HttpPlaybookBuilder(dependencyIdentifier)
              .post(CALIBRATION_VALIDATE_PATH)
              .willReturn(HttpResponse.serverError())
              .expectCalledAtLeast(1));
    }
  }

  static final class ScopedResetPlaybook extends ManagedHttpPlaybook {
    ScopedResetPlaybook(String dependencyIdentifier) {
      super(
          "playbook-ext-scoped-reset",
          dependencyIdentifier,
          new HttpPlaybookBuilder(dependencyIdentifier)
              .get("/api/v1/health")
              .willReturn(HttpResponse.okJson(Map.of("ok", true)))
              .expectNeverCalled());
    }
  }

  @RegisterExtension
  static final HttpPlaybookArenaFixture playbookArena = new HttpPlaybookArenaFixture();

  @Test
  @arena.junit.Playbook(ScopedOutagePlaybook.class)
  void scopedPlaybook_singleAnnotation_opensPlaybook() {
    assertNotNull(playbookArena.openArena().playbook(ScopedOutagePlaybook.class));
  }

  @Test
  @arena.junit.Playbook(ScopedOutagePlaybook.class)
  @arena.junit.Playbook(ScopedResetPlaybook.class)
  void scopedPlaybook_stackedAnnotations_opensBothPlaybooks() {
    OpenArena arena = playbookArena.openArena();
    assertNotNull(arena.playbook(ScopedOutagePlaybook.class));
    assertNotNull(arena.playbook(ScopedResetPlaybook.class));
  }

  @Test
  void scopedPlaybook_sessionDefaultRegistered_rejectsScopedActivation() {
    OpenArena arena = playbookArena.openArena();
    assertThrows(
        IllegalStateException.class,
        () -> openScopedPlaybooks(arena, SessionDefaultPlaybook.class));
  }

  private static void openScopedPlaybooks(OpenArena arena, Class<? extends Playbook>... classes) {
    for (Class<? extends Playbook> klass : classes) {
      Playbook pb = arena.playbook(klass);
      if (pb == null) {
        throw new IllegalStateException(
            "@Playbook: no playbook of class "
                + klass.getName()
                + " is registered on any match");
      }
      Boolean execOnDependencyStart = arena.playbookExecOnDependencyStart(klass);
      if (Boolean.TRUE.equals(execOnDependencyStart)) {
        throw new IllegalStateException(
            "@Playbook: playbook "
                + klass.getName()
                + " was registered with execOnDependencyStart=true and cannot be scoped per-test");
      }
      pb.run(arena);
    }
  }
}
