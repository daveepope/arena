package dev.arena.junit.playbook;
import dev.arena.junit.OpenArena;
import dev.arena.junit.support.ArenaJson;

import com.fasterxml.jackson.databind.node.ArrayNode;
import com.fasterxml.jackson.databind.node.ObjectNode;
import com.sun.jna.Pointer;
import java.util.ArrayList;
import java.util.List;

public final class HttpPlaybookBuilder {
  private final String dependencyIdentifier;
  private final List<ObjectNode> mappings = new ArrayList<>();

  public HttpPlaybookBuilder(String dependencyIdentifier) {
    this.dependencyIdentifier = dependencyIdentifier;
  }

  public HttpPlaybookBuilder withMapping(
      String method,
      String urlPath,
      int status,
      Object jsonBody,
      Integer priority,
      Integer expectCalled,
      Integer expectCalledAtLeast,
      boolean expectNeverCalled) {
    ObjectNode response = ArenaJson.object();
    response.put("status", status);
    if (jsonBody != null) {
      response.set("json_body", ArenaJson.MAPPER.valueToTree(jsonBody));
    }
    ObjectNode m = ArenaJson.object();
    m.put("method", method.toUpperCase());
    m.put("url_path", urlPath);
    m.set("response", response);
    if (priority != null) {
      m.put("priority", priority.intValue());
    }
    int expectSet =
        (expectCalled != null ? 1 : 0)
            + (expectCalledAtLeast != null ? 1 : 0)
            + (expectNeverCalled ? 1 : 0);
    if (expectSet > 1) {
      throw new IllegalArgumentException(
          "withMapping accepts at most one of: expectCalled, expectCalledAtLeast, expectNeverCalled");
    }
    if (expectCalled != null) {
      ObjectNode ex = ArenaJson.object();
      ex.put("kind", "exactly");
      ex.put("count", expectCalled.intValue());
      m.set("expect", ex);
    } else if (expectCalledAtLeast != null) {
      ObjectNode ex = ArenaJson.object();
      ex.put("kind", "at_least");
      ex.put("count", expectCalledAtLeast.intValue());
      m.set("expect", ex);
    } else if (expectNeverCalled) {
      ObjectNode ex = ArenaJson.object();
      ex.put("kind", "never");
      m.set("expect", ex);
    }
    mappings.add(m);
    return this;
  }

  public HttpPlaybookBuilder withMapping(String method, String urlPath, int status) {
    return withMapping(method, urlPath, status, null, null, null, null, false);
  }

  public HttpPlaybookBuilder withMapping(String method, String urlPath, int status, Object jsonBody) {
    return withMapping(method, urlPath, status, jsonBody, null, null, null, false);
  }

  public HttpPlaybook build(OpenArena arena) {
    if (mappings.isEmpty()) {
      throw new IllegalArgumentException("HttpPlaybookBuilder requires at least one mapping");
    }
    ArrayNode arr = ArenaJson.array();
    for (ObjectNode m : mappings) {
      arr.add(m.deepCopy());
    }
    return new HttpPlaybook(arena, dependencyIdentifier, arr);
  }
}
