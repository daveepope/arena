package arena.junit.readings.test;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import arena.junit.playbook.ManagedHttpPlaybook;

import com.fasterxml.jackson.databind.node.ArrayNode;
import com.fasterxml.jackson.databind.node.ObjectNode;

import java.util.List;
import java.util.Map;
import org.junit.jupiter.api.Test;

final class ReadingsHttpPlaybookRegistrationTest {

  static final class ValidationPlaybook extends ManagedHttpPlaybook {
    ValidationPlaybook() {
      super(
          "pb-reg",
          "dep-http",
          List.of(mapping("POST", "/api/v1/validate", 200, Map.of("valid", true))));
    }
  }

  @Test
  void forRegisteredFfi_singleMapping_serializesHttpPlaybookShape() {
    ValidationPlaybook pb = new ValidationPlaybook();
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

  @Test
  void forRegisteredFfi_mappingExpect_serializesExpectShape() {
    ObjectNode row =
        (ObjectNode)
            ((ArrayNode)
                    new ExpectPlaybook()
                        .forRegisteredFfi()
                        .path("mappings"))
                .get(0);
    ObjectNode expect = (ObjectNode) row.path("expect");
    assertEquals("at_least", expect.path("kind").asText());
    assertEquals(1, expect.path("count").asLong());
  }

  static final class ExpectPlaybook extends ManagedHttpPlaybook {
    ExpectPlaybook() {
      super(
          "pb-expect",
          "dep-http",
          List.of(
              mapping(
                  "POST",
                  "/api/v1/validate",
                  200,
                  Map.of("valid", true),
                  Expect.calledAtLeast(1))));
    }
  }
}
