package arena.junit.readings;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.fasterxml.jackson.databind.node.ArrayNode;
import com.fasterxml.jackson.databind.node.ObjectNode;
import arena.junit.playbook.ManagedHttpPlaybookBuilder;
import java.util.Map;
import org.junit.jupiter.api.Test;

final class ReadingsHttpPlaybookRegistrationTest {

  @Test
  void registeredHttpPlaybookFfiJsonUsesPlaybookShape() {
    var pb =
        new ManagedHttpPlaybookBuilder("pb-reg", "dep-http")
            .withMapping("POST", "/api/v1/validate", 200, Map.of("valid", true))
            .build();
    ObjectNode n = pb.forRegisteredFfi();
    assertEquals("http", n.path("kind").asText());
    assertEquals("pb-reg", n.path("identifier").asText());
    assertEquals("dep-http", n.path("dependency_identifier").asText());
    ArrayNode mappings = (ArrayNode) n.path("mappings");
    assertEquals(1, mappings.size());
    ObjectNode row = (ObjectNode) mappings.get(0);
    assertEquals("POST", row.path("method").asText());
    assertEquals("/api/v1/validate", row.path("url_path").asText());
    assertEquals(200, row.path("status").asInt());
    assertTrue(row.has("json_body"));
  }
}
