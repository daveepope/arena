package arena.junit.readings.test;

import static arena.junit.readings.fixture.ReadingsArenaConfig.CALIBRATION_VALIDATE_PATH;
import static arena.junit.readings.fixture.ReadingsArenaConfig.KAFKA_PORT;
import static arena.junit.readings.fixture.ReadingsArenaConfig.auth;
import static arena.junit.readings.fixture.ReadingsArenaConfig.baseUrlDocker;
import static arena.junit.readings.fixture.ReadingsArenaConfig.baseUrlExec;
import static arena.junit.readings.fixture.ReadingsArenaConfig.consumeReadingCreated;
import static arena.junit.readings.fixture.ReadingsArenaConfig.createReading;
import static arena.junit.readings.fixture.ReadingsArenaConfig.getReadings;
import static arena.junit.readings.fixture.ReadingsArenaConfig.readingsClient;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.junit.jupiter.api.Assumptions.assumeTrue;

import arena.junit.Playbook;
import arena.junit.ffi.ArenaBindingError;
import arena.junit.playbook.ActiveHttpPlaybook;
import arena.junit.readings.fixture.ReadingsArenaConfig;
import arena.junit.readings.fixture.ReadingsArenaFixture;
import arena.junit.readings.playbook.CalibrationOutagePlaybook;
import arena.junit.readings.playbook.CalibrationOutageVerifyProbePlaybook;
import arena.junit.readings.playbook.ResetValidationDbPlaybook;
import com.fasterxml.jackson.databind.JsonNode;
import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.time.Duration;
import java.util.ArrayList;
import java.util.List;
import java.util.concurrent.ArrayBlockingQueue;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.TimeUnit;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.extension.RegisterExtension;

final class ReadingsAxumWorkflowComponentTest {

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
        readingsArena
            .openArena()
            .playbook(CalibrationOutageVerifyProbePlaybook.class);
    try (ActiveHttpPlaybook active = (ActiveHttpPlaybook) pb.run(readingsArena.openArena())) {
      assertThrows(
          ArenaBindingError.class,
          () -> active.verify("POST", CALIBRATION_VALIDATE_PATH, 1));
    }
  }

  @Test
  @Playbook(ResetValidationDbPlaybook.class)
  void containerizedAppCreateReadingPublishesKafkaEvent() throws Exception {
    assumeTrue(readingsArena.containerizedWebEnabled());
    HttpClient c = ReadingsArenaConfig.readingsClient();
    String token = readingsArena.accessToken();
    String base = baseUrlDocker();
    ArrayBlockingQueue<Integer> idQueue = new ArrayBlockingQueue<>(1);
    List<Object> holder = new ArrayList<>(1);
    ExecutorService pool = Executors.newSingleThreadExecutor();
    pool.submit(
        () -> {
          try {
            int id = idQueue.take();
            JsonNode ev = consumeReadingCreated("localhost:" + KAFKA_PORT, id, "ctr");
            holder.add(ev);
          } catch (Exception e) {
            holder.add(e);
          }
        });
    int created =
        createReading(c, base, token, "Container Test User", 42, "test comment");
    idQueue.put(created);
    pool.shutdown();
    assertTrue(pool.awaitTermination(15, TimeUnit.SECONDS));
    assertEquals(1, holder.size());
    Object got = holder.get(0);
    if (got instanceof Exception ex) {
      throw ex;
    }
    JsonNode ev = (JsonNode) got;
    assertEquals(created, ev.path("id").asInt());
  }

  @Test
  @Playbook(ResetValidationDbPlaybook.class)
  void createReadingPublishesKafkaEventAndListsViaHttp() throws Exception {
    HttpClient c = readingsClient();
    String token = readingsArena.accessToken();
    String base = baseUrlExec();
    ArrayBlockingQueue<Integer> idQueue = new ArrayBlockingQueue<>(1);
    List<Object> holder = new ArrayList<>(1);
    ExecutorService pool = Executors.newSingleThreadExecutor();
    pool.submit(
        () -> {
          try {
            int id = idQueue.take();
            JsonNode ev = consumeReadingCreated("localhost:" + KAFKA_PORT, id, "exec");
            holder.add(ev);
          } catch (Exception e) {
            holder.add(e);
          }
        });
    int created =
        createReading(c, base, token, "Exec Test User", 42, "test comment");
    idQueue.put(created);
    pool.shutdown();
    assertTrue(pool.awaitTermination(15, TimeUnit.SECONDS));
    assertEquals(1, holder.size());
    Object got = holder.get(0);
    if (got instanceof Exception ex) {
      throw ex;
    }
    JsonNode ev = (JsonNode) got;
    assertEquals(created, ev.path("id").asInt());
    assertEquals("Exec Test User", ev.path("user_name").asText());
    assertEquals(42, ev.path("value").asInt());
    assertEquals("test comment", ev.path("comment").asText());
    List<JsonNode> readings = getReadings(c, base, token);
    assertTrue(readings.stream().anyMatch(r -> r.path("id").asInt() == created));
  }

  @Test
  @Playbook(ResetValidationDbPlaybook.class)
  void createMultipleReadingsAreListed() throws Exception {
    HttpClient c = readingsClient();
    String token = readingsArena.accessToken();
    String base = baseUrlExec();
    int id1 = createReading(c, base, token, "Bending", 1, "");
    int id2 =
        createReading(
            c, base, token, "joe", 2, "We're going to need a bigger ship");
    List<JsonNode> readings = getReadings(c, base, token);
    assertTrue(readings.stream().anyMatch(r -> r.path("id").asInt() == id1));
    assertTrue(readings.stream().anyMatch(r -> r.path("id").asInt() == id2));
  }
}
