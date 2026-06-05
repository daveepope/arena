package arena.junit.playbook;

import arena.junit.OpenArena;
import arena.junit.ffi.ArenaBindings;
import arena.junit.support.ArenaJson;

import com.fasterxml.jackson.databind.node.ArrayNode;
import com.fasterxml.jackson.databind.node.ObjectNode;
import com.sun.jna.Pointer;

import java.util.ArrayList;
import java.util.List;
import java.util.function.Function;

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
  private final List<ObjectNode> mappingNodes;

  protected ManagedHttpPlaybook(
      String identifier, String dependencyIdentifier, Iterable<Mapping> mappings) {
    this(identifier, dependencyIdentifier, mappingNodesFromLegacy(mappings));
  }

  protected ManagedHttpPlaybook(
      String identifier, String dependencyIdentifier, HttpSequenceBuilder sequence) {
    this(identifier, dependencyIdentifier, sequence.intoPlaybook().mappingsForFfi());
  }

  protected ManagedHttpPlaybook(
      String identifier, String dependencyIdentifier, HttpPlaybookBuilder builder) {
    this(identifier, dependencyIdentifier, builder.mappingsForFfi());
  }

  private ManagedHttpPlaybook(
      String identifier, String dependencyIdentifier, List<ObjectNode> mappingNodes) {
    if (identifier == null || identifier.isEmpty()) {
      throw new IllegalArgumentException("ManagedHttpPlaybook requires an identifier");
    }
    if (dependencyIdentifier == null || dependencyIdentifier.isEmpty()) {
      throw new IllegalArgumentException("ManagedHttpPlaybook requires a dependency identifier");
    }
    if (mappingNodes == null || mappingNodes.isEmpty()) {
      throw new IllegalArgumentException("ManagedHttpPlaybook requires at least one mapping");
    }
    this.identifier = identifier;
    this.dependencyIdentifier = dependencyIdentifier;
    this.mappingNodes = List.copyOf(mappingNodes);
  }

  private static List<ObjectNode> mappingNodesFromLegacy(Iterable<Mapping> mappings) {
    if (mappings == null) {
      throw new IllegalArgumentException("ManagedHttpPlaybook requires at least one mapping");
    }
    List<ObjectNode> nodes = new ArrayList<>();
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
      nodes.add(mn);
    }
    if (nodes.isEmpty()) {
      throw new IllegalArgumentException("ManagedHttpPlaybook requires at least one mapping");
    }
    return nodes;
  }

  public static ManagedHttpPlaybook fromBuilder(
      String identifier,
      String dependencyIdentifier,
      Function<HttpPlaybookBuilder, ?> build) {
    HttpPlaybookBuilder builder = new HttpPlaybookBuilder(dependencyIdentifier);
    Object result = build.apply(builder);
    HttpPlaybookBuilder built;
    if (result instanceof HttpSequenceBuilder seq) {
      built = seq.intoPlaybook();
    } else if (result instanceof HttpPlaybookBuilder playbookBuilder) {
      built = playbookBuilder;
    } else {
      throw new IllegalArgumentException(
          "HTTP playbook builder function must return HttpPlaybookBuilder or HttpSequenceBuilder");
    }
    return new ManagedHttpPlaybook(identifier, dependencyIdentifier, built.mappingsForFfi());
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
    for (ObjectNode m : mappingNodes) {
      arr.add(m.deepCopy());
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
