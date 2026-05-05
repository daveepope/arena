package arena.junit.playbook;
import arena.junit.OpenArena;
import arena.junit.ffi.ArenaBindingError;
import arena.junit.support.ArenaJson;

import com.fasterxml.jackson.databind.node.ArrayNode;
import com.fasterxml.jackson.databind.node.ObjectNode;
import com.sun.jna.Pointer;

public final class HttpPlaybook implements AutoCloseable {
  private final OpenArena arena;
  private final String dependencyIdentifier;
  private final ArrayNode mappings;
  private Pointer handle;

  HttpPlaybook(OpenArena arena, String dependencyIdentifier, ArrayNode mappings) {
    this.arena = arena;
    this.dependencyIdentifier = dependencyIdentifier;
    this.mappings = mappings;
  }

  public void open() {
    ObjectNode spec = ArenaJson.object();
    spec.put("dependency_identifier", dependencyIdentifier);
    spec.set("mappings", mappings);
    try {
      String json = ArenaJson.MAPPER.writeValueAsString(spec);
      handle = ArenaFfiPlaybooks.httpPlaybookOpen(arena, json);
    } catch (Exception e) {
      throw new ArenaBindingError(e.getMessage());
    }
  }

  @Override
  public void close() {
    Pointer h = handle;
    handle = null;
    if (h == null || Pointer.nativeValue(h) == 0) {
      return;
    }
    try {
      ArenaFfiPlaybooks.httpPlaybookClose(h);
    } catch (ArenaBindingError e) {
      throw new AssertionError(e.getMessage(), e);
    }
  }

  public void verify(String method, String urlPath, int expectedCount) {
    if (handle == null || Pointer.nativeValue(handle) == 0) {
      throw new IllegalStateException("HttpPlaybook.verify called outside of active playbook");
    }
    ObjectNode spec = ArenaJson.object();
    spec.put("dependency_identifier", dependencyIdentifier);
    spec.put("method", method.toUpperCase());
    spec.put("url_path", urlPath);
    spec.put("expected_count", expectedCount);
    try {
      ArenaFfiPlaybooks.httpPlaybookVerify(handle, ArenaJson.MAPPER.writeValueAsString(spec));
    } catch (Exception e) {
      throw new ArenaBindingError(e.getMessage());
    }
  }
}
