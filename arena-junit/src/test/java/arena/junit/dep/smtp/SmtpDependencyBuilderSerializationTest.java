package arena.junit.dep.smtp;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.fasterxml.jackson.databind.node.ObjectNode;

import org.junit.jupiter.api.Test;

final class SmtpDependencyBuilderSerializationTest {

  @Test
  void build_minimalName_serializesTypeAndIdentifier() {
    ObjectNode config = new SmtpDependencyBuilder("smtp").build().forFfi();
    assertEquals("smtp", config.path("type").asText());
    assertTrue(config.path("identifier").asText().startsWith("arena-smtp-smtp-"));
    assertFalse(config.has("image"));
    assertFalse(config.has("port"));
  }

  @Test
  void withOverrides_allFields_serializesConfiguredFields() {
    ObjectNode config =
        new SmtpDependencyBuilder("smtp")
            .withImage("v1.30.5")
            .withImageName("axllent/mailpit")
            .withPort(11025)
            .withUiPort(18025)
            .withContainerName("smtp-box")
            .build()
            .forFfi();
    assertEquals("v1.30.5", config.path("image").asText());
    assertEquals("axllent/mailpit", config.path("image_name").asText());
    assertEquals(11025, config.path("port").asInt());
    assertEquals(18025, config.path("ui_port").asInt());
    assertEquals("smtp-box", config.path("container_name").asText());
  }
}
