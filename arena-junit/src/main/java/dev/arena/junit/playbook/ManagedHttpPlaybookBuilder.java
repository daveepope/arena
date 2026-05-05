package dev.arena.junit.playbook;
import dev.arena.junit.support.ArenaJson;

import com.fasterxml.jackson.databind.node.ObjectNode;
import java.util.ArrayList;
import java.util.List;

public final class ManagedHttpPlaybookBuilder {
  private final String identifier;
  private final String dependencyIdentifier;
  private final List<ObjectNode> mappings = new ArrayList<>();

  public ManagedHttpPlaybookBuilder(String identifier, String dependencyIdentifier) {
    this.identifier = identifier;
    this.dependencyIdentifier = dependencyIdentifier;
  }

  public ManagedHttpPlaybookBuilder withMapping(
      String method, String urlPath, int status, Object jsonBody) {
    ObjectNode m = ArenaJson.object();
    m.put("method", method.toUpperCase());
    m.put("url_path", urlPath);
    m.put("status", status);
    if (jsonBody != null) {
      m.set("json_body", ArenaJson.MAPPER.valueToTree(jsonBody));
    }
    mappings.add(m);
    return this;
  }

  public ManagedHttpPlaybookBuilder withMapping(String method, String urlPath, int status) {
    return withMapping(method, urlPath, status, null);
  }

  public ManagedHttpPlaybook build() {
    if (mappings.isEmpty()) {
      throw new IllegalArgumentException("ManagedHttpPlaybookBuilder requires at least one mapping");
    }
    return new ManagedHttpPlaybook(identifier, dependencyIdentifier, List.copyOf(mappings));
  }
}
