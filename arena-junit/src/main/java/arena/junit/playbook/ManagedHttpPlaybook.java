package arena.junit.playbook;
import arena.junit.OpenArena;
import arena.junit.support.ArenaJson;

import com.fasterxml.jackson.databind.node.ArrayNode;
import com.fasterxml.jackson.databind.node.ObjectNode;
import java.util.List;

public final class ManagedHttpPlaybook implements ArenaPlaybookRegistration, ActivePlaybook {
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

  public HttpPlaybook activate(OpenArena arena) {
    ArrayNode out = ArenaJson.array();
    for (ObjectNode m : mappings) {
      ObjectNode row = ArenaJson.object();
      row.put("method", m.get("method").asText());
      row.put("url_path", m.get("url_path").asText());
      ObjectNode response = ArenaJson.object();
      int status = m.has("status") && !m.get("status").isNull() ? m.get("status").asInt() : 200;
      response.put("status", status);
      if (m.has("json_body") && !m.get("json_body").isNull()) {
        response.set("json_body", m.get("json_body").deepCopy());
      }
      row.set("response", response);
      row.put("priority", 1);
      out.add(row);
    }
    return new HttpPlaybook(arena, dependencyIdentifier, out);
  }

  @Override
  public AutoCloseable enter(OpenArena arena) {
    HttpPlaybook h = activate(arena);
    h.open();
    return h;
  }
}
