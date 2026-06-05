package arena.junit.playbook;

import arena.junit.support.ArenaJson;

import com.fasterxml.jackson.databind.node.ArrayNode;
import com.fasterxml.jackson.databind.node.ObjectNode;
import java.util.List;

public final class HttpMappingBuilder {
  private final HttpPlaybookBuilder playbook;
  private final ObjectNode spec;

  HttpMappingBuilder(HttpPlaybookBuilder playbook, String method, String urlPath) {
    this.playbook = playbook;
    spec = ArenaJson.object();
    spec.put("method", method.toUpperCase());
    spec.put("url_path", urlPath);
  }

  public HttpMappingBuilder withHeader(String name, ObjectNode pattern) {
    ObjectNode headers =
        spec.has("headers") ? (ObjectNode) spec.get("headers") : spec.putObject("headers");
    headers.set(name, pattern.deepCopy());
    return this;
  }

  public HttpMappingBuilder withRequestBody(Object body) {
    ArrayNode patterns =
        spec.has("body_patterns")
            ? (ArrayNode) spec.get("body_patterns")
            : spec.putArray("body_patterns");
    ObjectNode pattern = ArenaJson.object();
    try {
      pattern.put("equal_to_json", ArenaJson.MAPPER.writeValueAsString(body));
    } catch (Exception e) {
      throw new IllegalArgumentException("request body is not serializable: " + e.getMessage());
    }
    patterns.add(pattern);
    return this;
  }

  public HttpMappingBuilder withRequestBodyContaining(String substring) {
    ArrayNode patterns =
        spec.has("body_patterns")
            ? (ArrayNode) spec.get("body_patterns")
            : spec.putArray("body_patterns");
    ObjectNode pattern = ArenaJson.object();
    pattern.put("contains", substring);
    patterns.add(pattern);
    return this;
  }

  public HttpMappingBuilder withPriority(int priority) {
    spec.put("priority", priority);
    return this;
  }

  public HttpMappingBuilder inScenario(String name) {
    spec.put("scenario_name", name);
    return this;
  }

  public HttpMappingBuilder whenStateIs(String state) {
    spec.put("when_state_is", state);
    return this;
  }

  public HttpMappingBuilder willSetStateTo(String state) {
    spec.put("will_set_state_to", state);
    return this;
  }

  public HttpSequenceBuilder willReturn(HttpResponse response) {
    return new HttpSequenceBuilder(playbook, spec.deepCopy(), List.of(response));
  }

  public HttpSequenceBuilder willReturn(int status) {
    return willReturn(HttpResponse.status(status));
  }

  public HttpSequenceBuilder willReturn(int status, Object jsonBody) {
    return willReturn(HttpResponse.status(status).withJsonBody(jsonBody));
  }

  public HttpPlaybookBuilder willReturnInSequence(List<HttpResponse> responses) {
    if (responses == null || responses.isEmpty()) {
      throw new IllegalArgumentException("willReturnInSequence requires at least one response");
    }
    ObjectNode mapping = spec.deepCopy();
    ArrayNode arr = ArenaJson.array();
    for (HttpResponse response : responses) {
      arr.add(response.forSpec());
    }
    mapping.set("responses", arr);
    playbook.appendMapping(mapping);
    return playbook;
  }
}
