package arena.junit.playbook;

import arena.junit.OpenArena;
import arena.junit.ffi.ArenaBindingError;
import arena.junit.support.ArenaJson;

import com.fasterxml.jackson.databind.node.ObjectNode;
import com.sun.jna.Pointer;

public final class ActiveLocalstackPlaybook implements AutoCloseable {
  private final String dependencyIdentifier;
  private Pointer handle;

  ActiveLocalstackPlaybook(String dependencyIdentifier) {
    this.dependencyIdentifier = dependencyIdentifier;
  }

  public void begin(OpenArena arena) {
    ObjectNode spec = ArenaJson.object();
    spec.put("dependency_identifier", dependencyIdentifier);
    try {
      handle =
          ArenaFfiPlaybooks.localstackPlaybookBegin(
              arena, ArenaJson.MAPPER.writeValueAsString(spec));
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
    ArenaFfiPlaybooks.localstackPlaybookFinish(h);
  }
}
