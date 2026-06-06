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

  private KafkaWait() {}

  static JsonNode waitReadingCreatedDetail(
      ObjectMapper mapper, String bootstrap, String topic, int expectedId) throws Exception {
    Properties p = new Properties();
    p.put(ConsumerConfig.BOOTSTRAP_SERVERS_CONFIG, bootstrap);
    p.put(
        ConsumerConfig.GROUP_ID_CONFIG,
        "example-axum-junit-" + ProcessHandle.current().pid());
    p.put(ConsumerConfig.KEY_DESERIALIZER_CLASS_CONFIG, StringDeserializer.class.getName());
    p.put(ConsumerConfig.VALUE_DESERIALIZER_CLASS_CONFIG, StringDeserializer.class.getName());
    p.put(ConsumerConfig.AUTO_OFFSET_RESET_CONFIG, "earliest");
    try (KafkaConsumer<String, String> consumer = new KafkaConsumer<>(p)) {
      consumer.subscribe(List.of(topic));
      long deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(5);
      while (System.nanoTime() < deadline) {
        ConsumerRecords<String, String> recs = consumer.poll(Duration.ofMillis(100));
        for (ConsumerRecord<String, String> r : recs) {
          if (r.value() == null) {
            continue;
          }
          JsonNode ev = mapper.readTree(r.value());
          if (ev.path("id").asInt(-1) == expectedId) {
            return ev;
          }
        }
      }
    }
    throw new AssertionError("did not receive ReadingCreatedEvent before timeout");
  }
}
