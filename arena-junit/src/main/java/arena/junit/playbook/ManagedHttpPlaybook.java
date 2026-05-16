package arena.junit.playbook;

import arena.junit.OpenArena;
import arena.junit.support.ArenaJson;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.node.ArrayNode;
import com.fasterxml.jackson.databind.node.ObjectNode;

import java.util.List;

public final class ManagedHttpPlaybook implements ArenaPlaybookRegistration, Playbook {
  private final String identifier;
  private final String dependencyIdentifier;
  private final List<ObjectNode> mappings;

  ManagedHttpPlaybook(String identifier, String dependencyIdentifier, List<ObjectNode> mappings) {
    this.identifier = identifier;
    this.dependencyIdentifier = dependencyIdentifier;
    this.mappings = mappings;
  }

  public String identifier() {
    return identifier;
  }

  @Override
  public ObjectNode forRegisteredFfi() {
    ObjectNode n = ArenaJson.object();
    n.put("identifier", identifier);
    n.put("kind", "http");
    n.put("dependency_identifier", dependencyIdentifier);
    ArrayNode arr = ArenaJson.array();
    for (ObjectNode m : mappings) {
      arr.add(m.deepCopy());
    }
    n.set("mappings", arr);
    return n;
  }

  public ActiveHttpPlaybook run(OpenArena arena) {
    ActiveHttpPlaybookBuilder b = new ActiveHttpPlaybookBuilder(dependencyIdentifier);
    for (ObjectNode m : mappings) {
      int status = m.has("status") && !m.get("status").isNull() ? m.get("status").asInt() : 200;
      JsonNode jb = m.get("json_body");
      Object jsonBody = null;
      if (jb != null && !jb.isNull()) {
        try {
          jsonBody = ArenaJson.MAPPER.treeToValue(jb, Object.class);
        } catch (Exception e) {
          throw new IllegalArgumentException("ManagedHttpPlaybook: invalid json_body", e);
        }
      }
      b.withMapping(
          m.get("method").asText(),
          m.get("url_path").asText(),
          status,
          jsonBody,
          Integer.valueOf(1),
          null,
          null,
          false);
    }
    return b.build(arena);
  }

  @Override
  public AutoCloseable enter(OpenArena arena) {
    ActiveHttpPlaybook h = run(arena);
    h.begin();
    return h;
  }
}
