package arena.junit.playbook;

import arena.junit.OpenArena;
import arena.junit.ffi.ArenaBindings;
import arena.junit.ffi.ArenaBindingError;
import arena.junit.support.ArenaJson;

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

  public HttpMappingBuilder get(String urlPath) {
    return new HttpMappingBuilder(this, "GET", urlPath);
  }

  public HttpMappingBuilder post(String urlPath) {
    return new HttpMappingBuilder(this, "POST", urlPath);
  }

  public HttpMappingBuilder put(String urlPath) {
    return new HttpMappingBuilder(this, "PUT", urlPath);
  }

  public HttpMappingBuilder delete(String urlPath) {
    return new HttpMappingBuilder(this, "DELETE", urlPath);
  }

  void appendMapping(ObjectNode mapping) {
    mappings.add(mapping.deepCopy());
  }

  public List<ObjectNode> mappingsForFfi() {
    if (mappings.isEmpty()) {
      throw new IllegalArgumentException("HttpPlaybookBuilder requires at least one mapping");
    }
    List<ObjectNode> out = new ArrayList<>();
    for (ObjectNode m : mappings) {
      out.add(m.deepCopy());
    }
    return out;
  }

  public HttpPlaybookBuilder intoPlaybook() {
    return this;
  }

  public ActiveHttpPlaybook open(OpenArena arena) {
    ArrayNode arr = ArenaJson.array();
    for (ObjectNode m : mappingsForFfi()) {
      arr.add(m);
    }
    ObjectNode spec = ArenaJson.object();
    spec.put("dependency_identifier", dependencyIdentifier);
    spec.set("mappings", arr);
    try {
      Pointer handle =
          ArenaBindings.httpPlaybookOpen(arena.handle(), ArenaJson.MAPPER.writeValueAsString(spec));
      return new ActiveHttpPlaybook(handle);
    } catch (Exception e) {
      throw new ArenaBindingError(e.getMessage());
    }
  }
}
