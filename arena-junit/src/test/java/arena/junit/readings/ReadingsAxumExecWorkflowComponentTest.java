package arena.junit.readings;

import static arena.junit.readings.ReadingsArenaConfig.KAFKA_PORT;
import static arena.junit.readings.ReadingsArenaConfig.baseUrlExec;
import static arena.junit.readings.ReadingsArenaConfig.consumeReadingCreated;
import static arena.junit.readings.ReadingsArenaConfig.createReading;
import static arena.junit.readings.ReadingsArenaConfig.getReadings;
import static arena.junit.readings.ReadingsArenaConfig.readingsClient;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.fasterxml.jackson.databind.JsonNode;
import arena.junit.playbook.ArenaPlaybooks;
import java.net.http.HttpClient;
import java.util.ArrayList;
import java.util.List;
import java.util.concurrent.ArrayBlockingQueue;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.TimeUnit;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.extension.RegisterExtension;

final class ReadingsAxumExecWorkflowComponentTest {

  @RegisterExtension
  static final ReadingsArenaSessionFixture readingsArena = new ReadingsArenaSessionFixture();

  @Test
  @ArenaPlaybooks(ReadingsAxumValidationDbPlaybooks.class)
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
  @ArenaPlaybooks(ReadingsAxumValidationDbPlaybooks.class)
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
