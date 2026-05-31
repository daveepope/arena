package arena.junit.playbook;

import arena.junit.OpenArena;
import arena.junit.ffi.ArenaBindings;
import arena.junit.support.ArenaJson;

import com.fasterxml.jackson.databind.node.ArrayNode;
import com.fasterxml.jackson.databind.node.ObjectNode;
import com.sun.jna.Pointer;

import java.util.List;

public class ManagedHttpPlaybook implements Playbook, PlaybookRegistration {

  public record Expect(String kind, Long count) {
    public static Expect called(long count) {
      return new Expect("exactly", count);
    }

    public static Expect calledAtLeast(long count) {
      return new Expect("at_least", count);
    }

    public static Expect neverCalled() {
      return new Expect("never", null);
    }
  }

  public record Mapping(String method, String urlPath, int status, Object jsonBody, Expect expect) {
    public Mapping {
      if (method == null || method.isEmpty()) {
        throw new IllegalArgumentException("HTTP playbook mapping requires a method");
      }
      if (urlPath == null || urlPath.isEmpty()) {
        throw new IllegalArgumentException("HTTP playbook mapping requires a url path");
      }
    }

    public Mapping(String method, String urlPath, int status) {
      this(method, urlPath, status, null, null);
    }

    public Mapping(String method, String urlPath, int status, Object jsonBody) {
      this(method, urlPath, status, jsonBody, null);
    }
  }

  public static Mapping mapping(String method, String urlPath, int status) {
    return new Mapping(method, urlPath, status);
  }

  public static Mapping mapping(String method, String urlPath, int status, Object jsonBody) {
    return new Mapping(method, urlPath, status, jsonBody);
  }

  public static Mapping mapping(
      String method, String urlPath, int status, Object jsonBody, Expect expect) {
    return new Mapping(method, urlPath, status, jsonBody, expect);
  }

  public static Mapping mapping(String method, String urlPath, int status, Expect expect) {
    return new Mapping(method, urlPath, status, null, expect);
  }

  private final String identifier;
  private final String dependencyIdentifier;
  private final List<Mapping> mappings;

  protected ManagedHttpPlaybook(
      String identifier, String dependencyIdentifier, List<Mapping> mappings) {
    if (identifier == null || identifier.isEmpty()) {
      throw new IllegalArgumentException("ManagedHttpPlaybook requires an identifier");
    }
    if (dependencyIdentifier == null || dependencyIdentifier.isEmpty()) {
      throw new IllegalArgumentException("ManagedHttpPlaybook requires a dependency identifier");
    }
    if (mappings == null || mappings.isEmpty()) {
      throw new IllegalArgumentException("ManagedHttpPlaybook requires at least one mapping");
    }
    this.identifier = identifier;
    this.dependencyIdentifier = dependencyIdentifier;
    this.mappings = List.copyOf(mappings);
  }

  @Override
  public String identifier() {
    return identifier;
  }

  public String dependencyIdentifier() {
    return dependencyIdentifier;
  }

  @Override
  public ObjectNode forRegisteredFfi() {
    ObjectNode n = ArenaJson.object();
    n.put("identifier", identifier);
    n.put("kind", "http");
    n.put("dependency_identifier", dependencyIdentifier);
    ArrayNode arr = ArenaJson.array();
    for (Mapping m : mappings) {
      ObjectNode mn = ArenaJson.object();
      mn.put("method", m.method().toUpperCase());
      mn.put("url_path", m.urlPath());
      mn.put("status", m.status());
      if (m.jsonBody() != null) {
        mn.set("json_body", ArenaJson.MAPPER.valueToTree(m.jsonBody()));
      }
      if (m.expect() != null) {
        ObjectNode en = ArenaJson.object();
        en.put("kind", m.expect().kind());
        if (m.expect().count() != null) {
          en.put("count", m.expect().count());
        }
        mn.set("expect", en);
      }
      arr.add(mn);
    }
    n.set("mappings", arr);
    return n;
  }

  @Override
  public ActiveHttpPlaybook run(OpenArena arena) {
    Pointer handle = ArenaBindings.matchPlaybookRun(arena.handle(), identifier);
    return new ActiveHttpPlaybook(handle);
  }
}
