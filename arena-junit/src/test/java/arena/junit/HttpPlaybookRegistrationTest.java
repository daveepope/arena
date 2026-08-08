package arena.junit;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import arena.junit.playbook.ActiveHttpPlaybookBuilder;
import arena.junit.playbook.HttpPlaybookBuilder;
import arena.junit.playbook.HttpResponse;
import arena.junit.playbook.ManagedHttpPlaybook;
import com.fasterxml.jackson.databind.node.ArrayNode;
import com.fasterxml.jackson.databind.node.ObjectNode;
import java.util.List;
import java.util.Map;
import org.junit.jupiter.api.Test;

final class HttpPlaybookRegistrationTest {

  static final class ValidationPlaybook extends ManagedHttpPlaybook {
    ValidationPlaybook() {
      super(
          "pb-reg",
          "dep-http",
          new HttpPlaybookBuilder("dep-http")
              .post("/api/v1/validate")
              .willReturn(HttpResponse.okJson(Map.of("valid", true))));
    }
  }

  @Test
  void activatesBeforeTest_always_returnsTrue() {
    assertTrue(new ValidationPlaybook().activatesBeforeTest());
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
    ObjectNode response = (ObjectNode) row.path("response");
    assertEquals(200, response.path("status").asInt());
    assertTrue(response.has("json_body"));
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
          new HttpPlaybookBuilder("dep-http")
              .post("/api/v1/validate")
              .willReturn(HttpResponse.okJson(Map.of("valid", true)))
              .expectCalledAtLeast(1));
    }
  }

  static final class LegacyValidationPlaybook extends ManagedHttpPlaybook {
    LegacyValidationPlaybook() {
      super(
          "pb-legacy",
          "dep-http",
          List.of(mapping("POST", "/api/v1/validate", 200, Map.of("valid", true))));
    }
  }

  @Test
  void legacyWithMapping_buildsResponseSpec() {
    ActiveHttpPlaybookBuilder builder = new ActiveHttpPlaybookBuilder("dep-http");
    builder.withMapping("POST", "/api/x", 500, null, null, null, 1, false);
    ObjectNode row = builder.mappingsForFfi().getFirst();
    assertEquals(500, row.path("response").path("status").asInt());
    assertEquals("at_least", row.path("expect").path("kind").asText());
  }

  @Test
  void legacyMapping_forRegisteredFfi_serializesFlatStatusShape() {
    ObjectNode row =
        (ObjectNode)
            ((ArrayNode)
                    new LegacyValidationPlaybook()
                        .forRegisteredFfi()
                        .path("mappings"))
                .get(0);
    assertEquals(200, row.path("status").asInt());
    assertTrue(row.has("json_body"));
  }
}
