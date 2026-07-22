package arena.junit.dep.temporal;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.fasterxml.jackson.databind.node.ObjectNode;

import org.junit.jupiter.api.Test;

final class TemporalDependencyBuilderSerializationTest {

  @Test
  void build_minimalName_serializesTypeAndIdentifier() {
    ObjectNode config = new TemporalDependencyBuilder("temporal").build().forFfi();
    assertEquals("temporal", config.path("type").asText());
    assertTrue(config.path("identifier").asText().startsWith("arena-temporal-temporal-"));
    assertFalse(config.has("image"));
    assertFalse(config.has("port"));
  }

  @Test
  void withOverrides_allFields_serializesConfiguredFields() {
    ObjectNode config =
        new TemporalDependencyBuilder("temporal")
            .withImage("1.24.2")
            .withImageName("temporalio/auto-setup")
            .withPort(17233)
            .withUiPort(18233)
            .withContainerName("temporal-box")
            .build()
            .forFfi();
    assertEquals("1.24.2", config.path("image").asText());
    assertEquals("temporalio/auto-setup", config.path("image_name").asText());
    assertEquals(17233, config.path("port").asInt());
    assertEquals(18233, config.path("ui_port").asInt());
    assertEquals("temporal-box", config.path("container_name").asText());
  }
}
