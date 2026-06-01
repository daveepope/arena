package arena.junit.readings.test;

import static arena.junit.readings.fixture.ReadingsArenaConfig.CALIBRATION_VALIDATE_PATH;
import static arena.junit.readings.fixture.ReadingsArenaConfig.auth;
import static arena.junit.readings.fixture.ReadingsArenaConfig.baseUrlExec;
import static arena.junit.readings.fixture.ReadingsArenaConfig.createReading;
import static arena.junit.readings.fixture.ReadingsArenaConfig.getReadings;
import static arena.junit.readings.fixture.ReadingsArenaConfig.readingsClient;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import arena.junit.Playbook;
import arena.junit.ffi.ArenaBindingError;
import arena.junit.playbook.ActiveHttpPlaybook;
import arena.junit.readings.fixture.ReadingsArenaFixture;
import arena.junit.readings.playbook.CalibrationOutagePlaybook;
import arena.junit.readings.playbook.ResetValidationDbPlaybook;

import com.fasterxml.jackson.databind.JsonNode;
import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.time.Duration;
import java.util.List;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.extension.RegisterExtension;

final class ReadingsAxumCalibrationWorkflowComponentTest {

  @RegisterExtension
  static final ReadingsArenaFixture readingsArena = new ReadingsArenaFixture();

  @Test
  @Playbook(CalibrationOutagePlaybook.class)
  void postReading_calibrationApiReturns500_propagates500() throws Exception {
    HttpClient c = readingsClient();
    String token = readingsArena.accessToken();
    HttpResponse<String> r =
        c.send(
            auth(token)
                .uri(URI.create(baseUrlExec() + "/readings"))
                .header("Content-Type", "application/json")
                .POST(
                    HttpRequest.BodyPublishers.ofString(
                        "{\"user_name\":\"Outage Test User\",\"value\":99,\"comment\":null}"))
                .timeout(Duration.ofSeconds(10))
                .build(),
            HttpResponse.BodyHandlers.ofString());
    assertEquals(500, r.statusCode(), r.body());
  }

  @Test
  void createReading_defaultCalibration_listsCreatedReading() throws Exception {
    HttpClient c = readingsClient();
    String token = readingsArena.accessToken();
    int recovered =
        createReading(c, baseUrlExec(), token, "Recovery Test User", 17, "post-outage");
    List<JsonNode> readings = getReadings(c, baseUrlExec(), token);
    assertTrue(readings.stream().anyMatch(r -> r.path("id").asInt() == recovered));
  }

  @Test
  @Playbook(CalibrationOutagePlaybook.class)
  @Playbook(ResetValidationDbPlaybook.class)
  void postReading_outageAndValidationDbScopes_propagates500() throws Exception {
    HttpClient c = readingsClient();
    String token = readingsArena.accessToken();
    HttpResponse<String> r =
        c.send(
            auth(token)
                .uri(URI.create(baseUrlExec() + "/readings"))
                .header("Content-Type", "application/json")
                .POST(
                    HttpRequest.BodyPublishers.ofString(
                        "{\"user_name\":\"Scoped Outage\",\"value\":1,\"comment\":null}"))
                .timeout(Duration.ofSeconds(10))
                .build(),
            HttpResponse.BodyHandlers.ofString());
    assertEquals(500, r.statusCode(), r.body());
  }

  @Test
  void verify_calibrationOutageWithoutTraffic_throwsBindingError() {
    arena.junit.playbook.Playbook pb =
        readingsArena.openArena().playbook(CalibrationOutagePlaybook.class);
    try (ActiveHttpPlaybook active = (ActiveHttpPlaybook) pb.run(readingsArena.openArena())) {
      assertThrows(
          ArenaBindingError.class,
          () -> active.verify("POST", CALIBRATION_VALIDATE_PATH, 1));
    }
  }
}
