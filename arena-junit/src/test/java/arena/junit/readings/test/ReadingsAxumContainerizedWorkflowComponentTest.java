package arena.junit.readings.test;

import static arena.junit.readings.fixture.ReadingsArenaConfig.KAFKA_PORT;
import static arena.junit.readings.fixture.ReadingsArenaConfig.baseUrlDocker;
import static arena.junit.readings.fixture.ReadingsArenaConfig.consumeReadingCreated;
import static arena.junit.readings.fixture.ReadingsArenaConfig.createReading;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.junit.jupiter.api.Assumptions.assumeTrue;

import arena.junit.Playbook;
import arena.junit.readings.fixture.ReadingsArenaConfig;
import arena.junit.readings.fixture.ReadingsArenaFixture;
import arena.junit.readings.playbook.ResetValidationDbPlaybook;
import com.fasterxml.jackson.databind.JsonNode;
import java.net.http.HttpClient;
import java.util.ArrayList;
import java.util.List;
import java.util.concurrent.ArrayBlockingQueue;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.TimeUnit;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.extension.RegisterExtension;

final class ReadingsAxumContainerizedWorkflowComponentTest {

  @RegisterExtension
  static final ReadingsArenaFixture readingsArena = new ReadingsArenaFixture();

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
}
