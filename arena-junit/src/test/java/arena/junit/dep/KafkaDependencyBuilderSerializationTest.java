package arena.junit.dep;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import arena.junit.match.ArenaRunnableDependency;
import arena.junit.support.ArenaJson;

import com.fasterxml.jackson.databind.node.ArrayNode;
import com.fasterxml.jackson.databind.node.ObjectNode;

import org.junit.jupiter.api.Test;

final class KafkaDependencyBuilderSerializationTest {

  static final class StubDependency implements ArenaRunnableDependency {
    @Override
    public ObjectNode forFfi() {
      return ArenaJson.object().put("identifier", "child");
    }
  }

  @Test
  void build_minimalName_serializesTypeAndEmptyTopics() {
    ObjectNode config = new KafkaDependencyBuilder("kafka").build().forFfi();
    assertEquals("kafka", config.path("type").asText());
    assertTrue(config.path("identifier").asText().startsWith("arena-kafka-kafka-"));
    assertTrue(config.path("topics").isEmpty());
    assertFalse(config.has("children"));
  }

  @Test
  void withTopic_multipleCalls_appendsTopicsArray() {
    ObjectNode config =
        new KafkaDependencyBuilder("kafka").withTopic("orders").withTopic("payments").build().forFfi();
    ArrayNode topics = (ArrayNode) config.path("topics");
    assertEquals(2, topics.size());
    assertEquals("orders", topics.get(0).asText());
    assertEquals("payments", topics.get(1).asText());
  }

  @Test
  void withFlavor_confluent_serializesFlavorValue() {
    ObjectNode config =
        new KafkaDependencyBuilder("kafka").withFlavor(KafkaFlavor.CONFLUENT).build().forFfi();
    assertEquals("confluent", config.path("flavor").asText());
  }

  @Test
  void withFlavor_apacheNative_serializesFlavorValue() {
    ObjectNode config =
        new KafkaDependencyBuilder("kafka").withFlavor(KafkaFlavor.APACHE_NATIVE).build().forFfi();
    assertEquals("apache_native", config.path("flavor").asText());
  }

  @Test
  void withPortAndContainerName_setsScalarFields() {
    ObjectNode config =
        new KafkaDependencyBuilder("kafka")
            .withPort(19092)
            .withContainerName("kafka-box")
            .withImageName("apache/kafka")
            .build()
            .forFfi();
    assertEquals(19092, config.path("port").asInt());
    assertEquals("kafka-box", config.path("container_name").asText());
    assertEquals("apache/kafka", config.path("image_name").asText());
  }

  @Test
  void addChildDependency_nonEmptyChildren_serializesChildrenArray() {
    ObjectNode config =
        new KafkaDependencyBuilder("kafka").addChildDependency(new StubDependency()).build().forFfi();
    assertEquals(1, ((ArrayNode) config.path("children")).size());
  }

  @Test
  void identifier_returnsConfiguredIdentifier() {
    KafkaDependency dep = new KafkaDependencyBuilder("kafka").build();
    assertTrue(dep.identifier().startsWith("arena-kafka-kafka-"));
  }
}
