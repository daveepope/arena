package arena.junit;

import static org.junit.jupiter.api.Assertions.assertDoesNotThrow;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertSame;
import static org.junit.jupiter.api.Assertions.assertThrows;

import arena.examples.testruntime.EphemeralTestRuntime;
import arena.junit.dep.HttpDependency;
import arena.junit.dep.HttpDependencyBuilder;
import arena.junit.ffi.ArenaBindingError;
import arena.junit.oauth.OauthDependency;
import arena.junit.oauth.OauthDependencyBuilder;
import arena.junit.oauth.OauthLoopbackTls;
import arena.junit.playbook.ActiveHttpPlaybook;
import arena.junit.playbook.ActivePlaybook;
import arena.junit.playbook.HttpPlaybookBuilder;
import arena.junit.playbook.ManagedHttpPlaybook;
import arena.junit.playbook.ManagedPlaybook;
import arena.junit.playbook.Playbook;
import arena.junit.playbook.UnmanagedPlaybook;
import com.sun.jna.Pointer;
import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.util.ArrayDeque;
import java.util.List;
import java.util.Map;
import java.util.Queue;
import org.junit.jupiter.api.Test;

@Arena
final class PlaybookInvocationExtensionComponentTest {

  private static final EphemeralTestRuntime RT = EphemeralTestRuntime.get();
  private static final String CALIBRATION_VALIDATE_PATH = "/api/v1/validate";

  @ArenaDependency static final OauthDependency OAUTH = buildOauth();

  @ArenaDependency
  static final HttpDependency CALIBRATION =
      new HttpDependencyBuilder("playbook-ext-calibration").withPort(RT.calibrationHostPort).build();

  @ArenaPlaybook
  static final SessionDefaultPlaybook SESSION_DEFAULT =
      new SessionDefaultPlaybook(CALIBRATION.identifier());

  @ArenaPlaybook(execOnDependencyStart = false)
  static final ScopedOutagePlaybook SCOPED_OUTAGE = new ScopedOutagePlaybook(CALIBRATION.identifier());

  @ArenaPlaybook(execOnDependencyStart = false)
  static final ScopedResetPlaybook SCOPED_RESET = new ScopedResetPlaybook(CALIBRATION.identifier());

  @ArenaPlaybook(execOnDependencyStart = false)
  static final VerifyPlaybook VERIFY = new VerifyPlaybook(CALIBRATION.identifier());

  @ArenaPlaybook(execOnDependencyStart = false)
  static final UnmanagedProbePlaybook UNMANAGED_PROBE = new UnmanagedProbePlaybook();

  @ArenaPlaybook(execOnDependencyStart = false)
  static final ManagedProbePlaybook MANAGED_PROBE = new ManagedProbePlaybook();

  private static final Queue<String> PROBE_CALL_ORDER = new ArrayDeque<>();

  private static OauthDependency buildOauth() {
    OauthLoopbackTls.PemPair pem = OauthLoopbackTls.oauthLoopbackTlsPemPair();
    return new OauthDependencyBuilder("playbook-ext-oauth")
        .withPort(RT.oauthPort)
        .withListenIp("0.0.0.0")
        .withServerTlsPem(pem.certificatePem(), pem.privateKeyPem())
        .withMetadataBaseUrl(RT.oauthIssuer)
        .build();
  }

  static final class SessionDefaultPlaybook extends ManagedHttpPlaybook {
    SessionDefaultPlaybook(String dependencyIdentifier) {
      super(
          "playbook-ext-session-default",
          dependencyIdentifier,
          new HttpPlaybookBuilder(dependencyIdentifier)
              .post(CALIBRATION_VALIDATE_PATH)
              .willReturn(arena.junit.playbook.HttpResponse.okJson(Map.of("valid", true))));
    }
  }

  static final class ScopedOutagePlaybook extends ManagedHttpPlaybook {
    ScopedOutagePlaybook(String dependencyIdentifier) {
      super(
          "playbook-ext-scoped-outage",
          dependencyIdentifier,
          new HttpPlaybookBuilder(dependencyIdentifier)
              .post(CALIBRATION_VALIDATE_PATH)
              .willReturn(arena.junit.playbook.HttpResponse.serverError()));
    }
  }

  static final class ScopedResetPlaybook extends ManagedHttpPlaybook {
    ScopedResetPlaybook(String dependencyIdentifier) {
      super(
          "playbook-ext-scoped-reset",
          dependencyIdentifier,
          new HttpPlaybookBuilder(dependencyIdentifier)
              .get("/api/v1/health")
              .willReturn(arena.junit.playbook.HttpResponse.okJson(Map.of("ok", true)))
              .expectNeverCalled());
    }
  }

  static final class VerifyPlaybook extends ManagedHttpPlaybook {
    VerifyPlaybook(String dependencyIdentifier) {
      super(
          "playbook-ext-verify",
          dependencyIdentifier,
          new HttpPlaybookBuilder(dependencyIdentifier)
              .post(CALIBRATION_VALIDATE_PATH)
              .willReturn(arena.junit.playbook.HttpResponse.okJson(Map.of("valid", true))));
    }
  }

  static final class ProbeActivePlaybook extends ActivePlaybook {
    ProbeActivePlaybook() {
      super(Pointer.NULL);
    }
  }

  static final class UnmanagedProbePlaybook implements Playbook, UnmanagedPlaybook {
    @Override
    public String identifier() {
      return "unmanaged-probe";
    }

    @Override
    public ActivePlaybook run(OpenArena arena) {
      PROBE_CALL_ORDER.add("unmanaged");
      return new ProbeActivePlaybook();
    }
  }

  static final class ManagedProbePlaybook implements Playbook, ManagedPlaybook {
    @Override
    public String identifier() {
      return "managed-probe";
    }

    @Override
    public ActivePlaybook run(OpenArena arena) {
      PROBE_CALL_ORDER.add("managed");
      return new ProbeActivePlaybook();
    }
  }

  @Test
  @arena.junit.Playbook(UnmanagedProbePlaybook.class)
  @arena.junit.Playbook(ManagedProbePlaybook.class)
  void scopedPlaybook_managedAndUnmanagedStacked_unmanagedRunsBeforeManagedNotYetRun() {
    assertEquals(List.of("unmanaged"), List.copyOf(PROBE_CALL_ORDER));
  }

  @Test
  @arena.junit.Playbook(ScopedOutagePlaybook.class)
  void scopedPlaybook_singleAnnotation_injectsActiveHttpPlaybook(
      ActiveHttpPlaybook active, OpenArena arena) {
    assertNotNull(active);
    assertNotNull(arena.playbook(ScopedOutagePlaybook.class));
  }

  @Test
  @arena.junit.Playbook(ScopedOutagePlaybook.class)
  void scopedPlaybook_singleAnnotation_opensPlaybook(OpenArena arena) {
    assertNotNull(arena.playbook(ScopedOutagePlaybook.class));
  }

  @Test
  void arenaDependencyTypedParameter_resolvesToDeclaredFieldInstance(HttpDependency calibration) {
    assertSame(CALIBRATION, calibration);
  }

  @Test
  @arena.junit.Playbook(ScopedOutagePlaybook.class)
  @arena.junit.Playbook(ScopedResetPlaybook.class)
  void scopedPlaybook_stackedAnnotations_opensBothPlaybooks(OpenArena arena) {
    assertNotNull(arena.playbook(ScopedOutagePlaybook.class));
    assertNotNull(arena.playbook(ScopedResetPlaybook.class));
  }

  @Test
  void scopedPlaybook_sessionDefaultRegistered_rejectsScopedActivation(OpenArena arena) {
    assertThrows(
        IllegalStateException.class,
        () -> openScopedPlaybooks(arena, SessionDefaultPlaybook.class));
  }

  @Test
  void verifyAtLeast_withTraffic_succeeds(OpenArena arena) throws Exception {
    Playbook pb = arena.playbook(VerifyPlaybook.class);
    try (ActiveHttpPlaybook active = (ActiveHttpPlaybook) pb.run(arena)) {
      HttpClient client = HttpClient.newHttpClient();
      HttpResponse<Void> response =
          client.send(
              HttpRequest.newBuilder()
                  .uri(
                      URI.create(
                          "http://127.0.0.1:"
                              + RT.calibrationHostPort
                              + CALIBRATION_VALIDATE_PATH))
                  .POST(HttpRequest.BodyPublishers.ofString("{}"))
                  .header("Content-Type", "application/json")
                  .build(),
              HttpResponse.BodyHandlers.discarding());
      assertEquals(200, response.statusCode());
      active.verifyAtLeast("POST", CALIBRATION_VALIDATE_PATH, 1);
    }
  }

  @Test
  void verifyAtLeast_withoutTraffic_throwsBindingError(OpenArena arena) {
    Playbook pb = arena.playbook(VerifyPlaybook.class);
    try (ActiveHttpPlaybook active = (ActiveHttpPlaybook) pb.run(arena)) {
      assertThrows(
          ArenaBindingError.class,
          () -> active.verifyAtLeast("POST", CALIBRATION_VALIDATE_PATH, 1));
    }
  }

  @Test
  void verify_failure_closeDoesNotThrow(OpenArena arena) {
    Playbook pb = arena.playbook(VerifyPlaybook.class);
    ActiveHttpPlaybook active = (ActiveHttpPlaybook) pb.run(arena);
    assertThrows(
        ArenaBindingError.class,
        () -> active.verify("POST", CALIBRATION_VALIDATE_PATH, 1));
    assertDoesNotThrow(active::close);
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
