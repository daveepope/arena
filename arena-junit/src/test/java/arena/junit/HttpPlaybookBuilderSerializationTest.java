package arena.junit;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import arena.junit.playbook.HttpHeaderPattern;
import arena.junit.playbook.HttpPlaybookBuilder;
import arena.junit.playbook.HttpResponse;
import arena.junit.playbook.ManagedHttpPlaybook;

import com.fasterxml.jackson.databind.node.ArrayNode;
import com.fasterxml.jackson.databind.node.ObjectNode;

import java.util.List;
import java.util.Map;
import org.junit.jupiter.api.Test;

final class HttpPlaybookBuilderSerializationTest {

  @Test
  void thenReturn_multiResponse_serializesResponsesArray() {
    ObjectNode row =
        (ObjectNode)
            new HttpPlaybookBuilder("dep-id")
                .get("/api/x")
                .willReturn(HttpResponse.serverError())
                .thenReturn(HttpResponse.status(503))
                .thenReturn(HttpResponse.okJson(Map.of("ok", true)))
                .intoPlaybook()
                .mappingsForFfi()
                .getFirst();
    assertFalse(row.has("response"));
    ArrayNode responses = (ArrayNode) row.path("responses");
    assertEquals(500, responses.get(0).path("status").asInt());
    assertEquals(503, responses.get(1).path("status").asInt());
    assertTrue(responses.get(2).path("json_body").path("ok").asBoolean());
  }

  @Test
  void willReturnInSequence_multiResponse_serializesResponsesArray() {
    ObjectNode row =
        (ObjectNode)
            new HttpPlaybookBuilder("dep-id")
                .get("/api/x")
                .willReturnInSequence(
                    List.of(
                        HttpResponse.serverError(),
                        HttpResponse.status(503),
                        HttpResponse.okJson(Map.of("ok", true))))
                .mappingsForFfi()
                .getFirst();
    ArrayNode responses = (ArrayNode) row.path("responses");
    assertEquals(500, responses.get(0).path("status").asInt());
    assertTrue(responses.get(2).path("json_body").path("ok").asBoolean());
  }

  @Test
  void withHeaderAndBodyPatterns_serializesRequestMatchFields() {
    ObjectNode row =
        (ObjectNode)
            new HttpPlaybookBuilder("dep-id")
                .post("/api/x")
                .withHeader("Authorization", HttpHeaderPattern.matching("Bearer .+"))
                .withRequestBody(Map.of("command", "ignition"))
                .withRequestBodyContaining("ignite")
                .withPriority(2)
                .willReturn(HttpResponse.okJson(Map.of("accepted", true)))
                .intoPlaybook()
                .mappingsForFfi()
                .getFirst();
    assertEquals(2, row.path("priority").asInt());
    assertEquals(
        "Bearer .+",
        row.path("headers").path("Authorization").path("matches").asText());
    ArrayNode patterns = (ArrayNode) row.path("body_patterns");
    assertEquals("{\"command\":\"ignition\"}", patterns.get(0).path("equal_to_json").asText());
    assertEquals("ignite", patterns.get(1).path("contains").asText());
  }

  @Test
  void responseDelayAndHeaders_serializeInMappingSpec() {
    ObjectNode row =
        (ObjectNode)
            new HttpPlaybookBuilder("dep-id")
                .post("/api/x")
                .willReturn(
                    HttpResponse.created()
                        .withHeader("Location", "/api/x/1")
                        .withFixedDelayMs(30)
                        .withUniformRandomDelayMs(5, 15))
                .intoPlaybook()
                .mappingsForFfi()
                .getFirst();
    ObjectNode response = (ObjectNode) row.path("response");
    assertEquals(201, response.path("status").asInt());
    assertEquals("/api/x/1", response.path("headers").path("Location").asText());
    assertEquals(30, response.path("fixed_delay_ms").asLong());
    assertEquals("uniform", response.path("delay_distribution").path("type").asText());
  }

  @Test
  void expectCalled_exactCount_serializesExpectExactly() {
    ObjectNode row =
        (ObjectNode)
            new HttpPlaybookBuilder("dep-id")
                .post("/api/x")
                .willReturn(HttpResponse.okJson(Map.of("ok", true)))
                .expectCalled(2)
                .intoPlaybook()
                .mappingsForFfi()
                .getFirst();
    assertEquals("exactly", row.path("expect").path("kind").asText());
    assertEquals(2, row.path("expect").path("count").asLong());
  }

  @Test
  void inScenario_withStateFields_serializesScenarioShape() {
    ObjectNode row =
        (ObjectNode)
            new HttpPlaybookBuilder("dep-id")
                .get("/api/x")
                .inScenario("flow")
                .whenStateIs("ready")
                .willSetStateTo("done")
                .willReturn(HttpResponse.okJson(Map.of("step", 1)))
                .intoPlaybook()
                .mappingsForFfi()
                .getFirst();
    assertEquals("flow", row.path("scenario_name").asText());
    assertEquals("ready", row.path("when_state_is").asText());
    assertEquals("done", row.path("will_set_state_to").asText());
  }

  @Test
  void fromBuilder_fluentChain_preservesMappings() {
    ManagedHttpPlaybook playbook =
        ManagedHttpPlaybook.fromBuilder(
            "pb-from-builder",
            "dep-id",
            b -> b.get("/api/x").willReturn(HttpResponse.okJson(Map.of("ok", true))));
    ObjectNode row =
        (ObjectNode) ((ArrayNode) playbook.forRegisteredFfi().path("mappings")).get(0);
    assertEquals("GET", row.path("method").asText());
    assertTrue(row.path("response").path("json_body").path("ok").asBoolean());
  }
}
