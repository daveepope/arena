package arena.junit;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertSame;

import arena.examples.testruntime.EphemeralTestRuntime;
import arena.junit.dep.HttpDependency;
import arena.junit.dep.HttpDependencyBuilder;
import arena.junit.playbook.ActiveHttpPlaybook;
import arena.junit.playbook.HttpPlaybookBuilder;
import arena.junit.playbook.ManagedHttpPlaybook;
import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.util.Map;
import org.junit.jupiter.api.Test;
import org.junit.platform.suite.api.SelectClasses;
import org.junit.platform.suite.api.Suite;

final class ArenaSuiteSharingComponentTest {

  private static final EphemeralTestRuntime RT = EphemeralTestRuntime.get();
  private static final String VALIDATE_PATH = "/api/v1/validate";

  static int afterOpenCount;
  static OpenArena capturedArena;

  static final class ScopedValidatePlaybook extends ManagedHttpPlaybook {
    ScopedValidatePlaybook(String dependencyIdentifier) {
      super(
          "arena-suite-sharing-scoped-validate",
          dependencyIdentifier,
          new HttpPlaybookBuilder(dependencyIdentifier)
              .post(VALIDATE_PATH)
              .willReturn(arena.junit.playbook.HttpResponse.okJson(Map.of("valid", true))));
    }
  }

  @Suite
  @SelectClasses({FirstMember.class, SecondMember.class})
  @Arena
  static final class SharedSuite {
    @ArenaDependency
    static final HttpDependency CALIBRATION =
        new HttpDependencyBuilder("arena-suite-sharing-calibration")
            .withPort(RT.calibrationHostPort)
            .build();

    @ArenaPlaybook(execOnDependencyStart = false)
    static final ScopedValidatePlaybook SCOPED_VALIDATE =
        new ScopedValidatePlaybook(CALIBRATION.identifier());

    @ArenaAfterOpen
    static void afterOpen(OpenArena arena) {
      afterOpenCount++;
      capturedArena = arena;
    }
  }

  @Arena(SharedSuite.class)
  static final class FirstMember {
    @Test
    void arena_injectedFromSuite_isSharedSingleOpen(OpenArena arena) {
      assertSame(capturedArena, arena);
      assertEquals(1, afterOpenCount);
    }

    @Test
    @arena.junit.Playbook(ScopedValidatePlaybook.class)
    void scopedPlaybook_fromFirstMember_validatesAgainstSharedDependency(ActiveHttpPlaybook active)
        throws Exception {
      assertNotNull(active);
      HttpClient client = HttpClient.newHttpClient();
      HttpResponse<Void> response =
          client.send(
              HttpRequest.newBuilder()
                  .uri(URI.create("http://127.0.0.1:" + RT.calibrationHostPort + VALIDATE_PATH))
                  .POST(HttpRequest.BodyPublishers.ofString("{}"))
                  .header("Content-Type", "application/json")
                  .build(),
              HttpResponse.BodyHandlers.discarding());
      assertEquals(200, response.statusCode());
    }
  }

  @Arena(SharedSuite.class)
  static final class SecondMember {
    @Test
    void arena_injectedFromSuite_sameInstanceAsFirstMember(OpenArena arena) {
      assertSame(capturedArena, arena);
      assertEquals(1, afterOpenCount);
    }
  }
}
