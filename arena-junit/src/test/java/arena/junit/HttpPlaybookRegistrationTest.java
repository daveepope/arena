package arena.junit;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
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
  void legacyWithMapping_expectCalledBranch_serializesExactlyKind() {
    ActiveHttpPlaybookBuilder builder = new ActiveHttpPlaybookBuilder("dep-http");
    builder.withMapping("GET", "/api/x", 200, null, null, 2, null, false);
    ObjectNode row = builder.mappingsForFfi().getFirst();
    assertEquals("exactly", row.path("expect").path("kind").asText());
    assertEquals(2, row.path("expect").path("count").asLong());
  }

  @Test
  void legacyWithMapping_expectNeverCalledBranch_serializesNeverKind() {
    ActiveHttpPlaybookBuilder builder = new ActiveHttpPlaybookBuilder("dep-http");
    builder.withMapping("DELETE", "/api/x", 204, null, null, null, null, true);
    ObjectNode row = builder.mappingsForFfi().getFirst();
    assertEquals("never", row.path("expect").path("kind").asText());
    assertEquals("DELETE", row.path("method").asText());
  }

  @Test
  void legacyWithMapping_putMethodAndPriority_setsMethodAndPriority() {
    ActiveHttpPlaybookBuilder builder = new ActiveHttpPlaybookBuilder("dep-http");
    builder.withMapping("PUT", "/api/x", 200, null, 3, null, null, false);
    ObjectNode row = builder.mappingsForFfi().getFirst();
    assertEquals("PUT", row.path("method").asText());
    assertEquals(3, row.path("priority").asInt());
  }

  @Test
  void legacyWithMapping_moreThanOneExpectOption_throwsIllegalArgumentException() {
    ActiveHttpPlaybookBuilder builder = new ActiveHttpPlaybookBuilder("dep-http");
    assertThrows(
        IllegalArgumentException.class,
        () -> builder.withMapping("GET", "/api/x", 200, null, null, 1, 1, false));
  }

  @Test
  void legacyWithMapping_unsupportedMethod_throwsIllegalArgumentException() {
    ActiveHttpPlaybookBuilder builder = new ActiveHttpPlaybookBuilder("dep-http");
    assertThrows(
        IllegalArgumentException.class,
        () -> builder.withMapping("PATCH", "/api/x", 200, null, null, null, null, false));
  }

  @Test
  void legacyWithMapping_statusOnlyOverload_buildsResponseWithoutJsonBody() {
    ActiveHttpPlaybookBuilder builder = new ActiveHttpPlaybookBuilder("dep-http");
    builder.withMapping("GET", "/api/x", 204);
    ObjectNode row = builder.mappingsForFfi().getFirst();
    assertEquals(204, row.path("response").path("status").asInt());
  }

  @Test
  void legacyWithMapping_statusAndJsonBodyOverload_buildsResponseWithJsonBody() {
    ActiveHttpPlaybookBuilder builder = new ActiveHttpPlaybookBuilder("dep-http");
    builder.withMapping("POST", "/api/x", 201, Map.of("created", true));
    ObjectNode row = builder.mappingsForFfi().getFirst();
    assertTrue(row.path("response").path("json_body").path("created").asBoolean());
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
