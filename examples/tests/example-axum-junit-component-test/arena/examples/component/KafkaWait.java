package arena.examples.component;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import java.time.Duration;
import java.util.List;
import java.util.Properties;
import java.util.concurrent.TimeUnit;
import org.apache.kafka.clients.consumer.ConsumerConfig;
import org.apache.kafka.clients.consumer.ConsumerRecord;
import org.apache.kafka.clients.consumer.ConsumerRecords;
import org.apache.kafka.clients.consumer.KafkaConsumer;
import org.apache.kafka.common.serialization.StringDeserializer;

final class KafkaWait {

  private static final long TIMEOUT_NANOS = TimeUnit.SECONDS.toNanos(15);
  private static final long ASSIGNMENT_WARMUP_NANOS = TimeUnit.SECONDS.toNanos(3);

  private KafkaWait() {}

  @FunctionalInterface
  interface ReadingCreateAction {
    int create() throws Exception;
  }

  static JsonNode waitReadingCreatedDetail(
      ObjectMapper mapper, String bootstrap, String topic, ReadingCreateAction create)
      throws Exception {
    Properties p = new Properties();
    p.put(ConsumerConfig.BOOTSTRAP_SERVERS_CONFIG, bootstrap);
    p.put(
        ConsumerConfig.GROUP_ID_CONFIG,
        "example-axum-junit-" + ProcessHandle.current().pid());
    p.put(ConsumerConfig.KEY_DESERIALIZER_CLASS_CONFIG, StringDeserializer.class.getName());
    p.put(ConsumerConfig.VALUE_DESERIALIZER_CLASS_CONFIG, StringDeserializer.class.getName());
    p.put(ConsumerConfig.AUTO_OFFSET_RESET_CONFIG, "earliest");
    p.put(ConsumerConfig.ENABLE_AUTO_COMMIT_CONFIG, "false");
    try (KafkaConsumer<String, String> consumer = new KafkaConsumer<>(p)) {
      consumer.subscribe(List.of(topic));
      warmAssignment(consumer);
      int expectedId = create.create();
      long deadline = System.nanoTime() + TIMEOUT_NANOS;
      while (System.nanoTime() < deadline) {
        ConsumerRecords<String, String> recs = consumer.poll(Duration.ofMillis(100));
        for (ConsumerRecord<String, String> r : recs) {
          if (r.value() == null) {
            continue;
          }
          JsonNode ev = mapper.readTree(r.value());
          if (ev.path("id").asLong(-1) == expectedId) {
            return ev;
          }
        }
      }
    }
    throw new AssertionError("did not receive ReadingCreatedEvent before timeout");
  }

  private static void warmAssignment(KafkaConsumer<String, String> consumer) {
    long deadline = System.nanoTime() + ASSIGNMENT_WARMUP_NANOS;
    while (System.nanoTime() < deadline) {
      consumer.poll(Duration.ofMillis(100));
      if (!consumer.assignment().isEmpty()) {
        return;
      }
    }
    System.err.println(
        "WARN: kafka consumer partition assignment not ready after "
            + TimeUnit.NANOSECONDS.toSeconds(ASSIGNMENT_WARMUP_NANOS)
            + "s; continuing consume poll");
  }
}
