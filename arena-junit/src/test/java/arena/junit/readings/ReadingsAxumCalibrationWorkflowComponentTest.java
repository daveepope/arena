package arena.junit.readings;

import static arena.junit.readings.ReadingsArenaConfig.CALIBRATION_VALIDATE_PATH;
import static arena.junit.readings.ReadingsArenaConfig.auth;
import static arena.junit.readings.ReadingsArenaConfig.baseUrlExec;
import static arena.junit.readings.ReadingsArenaConfig.createReading;
import static arena.junit.readings.ReadingsArenaConfig.getReadings;
import static arena.junit.readings.ReadingsArenaConfig.readingsClient;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.fasterxml.jackson.databind.JsonNode;
import arena.junit.playbook.ActivePlaybooks;
import arena.junit.playbook.ActiveHttpPlaybook;
import arena.junit.playbook.ActiveHttpPlaybookBuilder;
import arena.junit.playbook.ManagedHttpPlaybook;
import arena.junit.playbook.ManagedHttpPlaybookBuilder;
import arena.junit.playbook.ManagedMssqlPlaybook;
import arena.junit.playbook.ManagedMssqlPlaybookBuilder;
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
  static final ReadingsArenaSessionFixture readingsArena = new ReadingsArenaSessionFixture();

  @Test
  void postReadingReturns500WhenCalibrationApiReturns500() throws Exception {
    HttpClient c = readingsClient();
    String token = readingsArena.accessToken();
    ActiveHttpPlaybook outage =
        new ActiveHttpPlaybookBuilder(readingsArena.calibrationIdentifier())
            .withMapping("POST", CALIBRATION_VALIDATE_PATH, 500, null, 1, 1, null, false)
            .build(readingsArena.arena());
    outage.begin();
    try {
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
    } finally {
      outage.close();
    }
    int recovered =
        createReading(c, baseUrlExec(), token, "Recovery Test User", 17, "post-outage");
    List<JsonNode> readings = getReadings(c, baseUrlExec(), token);
    assertTrue(readings.stream().anyMatch(r -> r.path("id").asInt() == recovered));
  }

  @Test
  void postReadingReturns500WhenCalibrationApiOverriddenByPlaybook() throws Exception {
    ManagedHttpPlaybook outage =
        new ManagedHttpPlaybookBuilder(
                ReadingsArenaConfig.PLAYBOOK_CALIBRATION_OUTAGE_MANAGED,
                readingsArena.calibrationIdentifier())
            .withMapping("POST", CALIBRATION_VALIDATE_PATH, 500)
            .build();
    ManagedMssqlPlaybook validationDb =
        new ManagedMssqlPlaybookBuilder(
                ReadingsArenaConfig.PLAYBOOK_VALIDATION_DB_SCOPED, readingsArena.mssqlIdentifier())
            .build();
    try (ActivePlaybooks ignored =
        ActivePlaybooks.open(readingsArena.arena(), outage, validationDb)) {
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
    int recovered =
        createReading(
            readingsClient(), baseUrlExec(), readingsArena.accessToken(), "Scoped Recovery", 2, null);
    List<JsonNode> readings =
        getReadings(readingsClient(), baseUrlExec(), readingsArena.accessToken());
    assertTrue(readings.stream().anyMatch(r -> r.path("id").asInt() == recovered));
  }

  @Test
  void postReadingReturns500UnderScopedPlaybookStack() throws Exception {
    ManagedHttpPlaybook outage =
        new ManagedHttpPlaybookBuilder(
                ReadingsArenaConfig.PLAYBOOK_CALIBRATION_OUTAGE_FIXTURE_SCOPE,
                readingsArena.calibrationIdentifier())
            .withMapping("POST", CALIBRATION_VALIDATE_PATH, 500)
            .build();
    ManagedMssqlPlaybook validationDb =
        new ManagedMssqlPlaybookBuilder(
                ReadingsArenaConfig.PLAYBOOK_VALIDATION_DB_SCOPED, readingsArena.mssqlIdentifier())
            .build();
    try (ActivePlaybooks ignored =
        ActivePlaybooks.open(readingsArena.arena(), outage, validationDb)) {
      HttpClient c = readingsClient();
      String token = readingsArena.accessToken();
      HttpResponse<String> r =
          c.send(
              auth(token)
                  .uri(URI.create(baseUrlExec() + "/readings"))
                  .header("Content-Type", "application/json")
                  .POST(
                      HttpRequest.BodyPublishers.ofString(
                          "{\"user_name\":\"Stack Outage\",\"value\":1,\"comment\":null}"))
                  .timeout(Duration.ofSeconds(10))
                  .build(),
              HttpResponse.BodyHandlers.ofString());
      assertEquals(500, r.statusCode(), r.body());
    }
  }

  @Test
  void httpPlaybookCloseFailsWhenCallExpectationUnmet() {
    ActiveHttpPlaybook unused =
        new ActiveHttpPlaybookBuilder(readingsArena.calibrationIdentifier())
            .withMapping("POST", CALIBRATION_VALIDATE_PATH, 500, null, 1, 1, null, false)
            .build(readingsArena.arena());
    unused.begin();
    assertThrows(AssertionError.class, unused::close);
  }
}
