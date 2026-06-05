package arena.junit;

import static org.junit.jupiter.api.Assertions.assertDoesNotThrow;
import static org.junit.jupiter.api.Assertions.assertThrows;

import arena.examples.readings.testruntime.ReadingsEphemeralTestRuntime;
import arena.junit.dep.HttpDependency;
import arena.junit.dep.HttpDependencyBuilder;
import arena.junit.ffi.ArenaBindingError;
import arena.junit.match.Match;
import arena.junit.match.MatchBuilder;
import arena.junit.oauth.OauthDependency;
import arena.junit.oauth.OauthDependencyBuilder;
import arena.junit.oauth.OauthLoopbackTls;
import arena.junit.playbook.ActiveHttpPlaybook;
import arena.junit.playbook.HttpPlaybookBuilder;
import arena.junit.playbook.ManagedHttpPlaybook;
import arena.junit.playbook.Playbook;
import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.util.List;
import java.util.Map;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.extension.RegisterExtension;

final class ActiveHttpPlaybookVerifyTest {

  private static final ReadingsEphemeralTestRuntime RT = ReadingsEphemeralTestRuntime.get();
  private static final String CALIBRATION_VALIDATE_PATH = "/api/v1/validate";

  static final class VerifyArenaFixture extends ClosedArenaExtension {
    @Override
    protected ClosedArena buildClosedArena() throws Exception {
      OauthLoopbackTls.PemPair pem = OauthLoopbackTls.oauthLoopbackTlsPemPair();
      OauthDependency oauth =
          new OauthDependencyBuilder("verify-oauth")
              .withPort(RT.oauthPort)
              .withListenIp("0.0.0.0")
              .withServerTlsPem(pem.certificatePem(), pem.privateKeyPem())
              .withMetadataBaseUrl(RT.oauthIssuer)
              .build();
      HttpDependency calibration =
          new HttpDependencyBuilder("verify-calibration")
              .withPort(RT.calibrationHostPort)
              .build();
      VerifyPlaybook verifyPlaybook = new VerifyPlaybook(calibration.identifier());
      Match match =
          new MatchBuilder("verify-match")
              .addDependency(oauth)
              .addDependency(calibration)
              .registerPlaybook(verifyPlaybook, false)
              .build();
      return new ClosedArena("verify-arena", List.of(match));
    }
  }

  static final class VerifyPlaybook extends ManagedHttpPlaybook {
    VerifyPlaybook(String dependencyIdentifier) {
      super(
          "verify-playbook",
          dependencyIdentifier,
          new HttpPlaybookBuilder(dependencyIdentifier)
              .post(CALIBRATION_VALIDATE_PATH)
              .willReturn(arena.junit.playbook.HttpResponse.okJson(Map.of("valid", true)))
              .expectCalledAtLeast(1));
    }
  }

  @RegisterExtension static final VerifyArenaFixture verifyArena = new VerifyArenaFixture();

  @Test
  void verifyAtLeast_withTraffic_succeeds() throws Exception {
    OpenArena arena = verifyArena.openArena();
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
      org.junit.jupiter.api.Assertions.assertEquals(200, response.statusCode());
      active.verifyAtLeast("POST", CALIBRATION_VALIDATE_PATH, 1);
    }
  }

  @Test
  void verifyAtLeast_withoutTraffic_throwsBindingError() {
    OpenArena arena = verifyArena.openArena();
    Playbook pb = arena.playbook(VerifyPlaybook.class);
    try (ActiveHttpPlaybook active = (ActiveHttpPlaybook) pb.run(arena)) {
      assertThrows(
          ArenaBindingError.class,
          () -> active.verifyAtLeast("POST", CALIBRATION_VALIDATE_PATH, 1));
    }
  }

  @Test
  void verify_failure_closeDoesNotThrow() {
    OpenArena arena = verifyArena.openArena();
    Playbook pb = arena.playbook(VerifyPlaybook.class);
    ActiveHttpPlaybook active = (ActiveHttpPlaybook) pb.run(arena);
    assertThrows(
        ArenaBindingError.class,
        () -> active.verify("POST", CALIBRATION_VALIDATE_PATH, 1));
    assertDoesNotThrow(active::close);
  }
}
