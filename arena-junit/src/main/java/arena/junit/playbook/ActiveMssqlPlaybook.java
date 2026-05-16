package arena.junit.playbook;

import arena.junit.OpenArena;
import arena.junit.ffi.ArenaBindingError;
import arena.junit.support.ArenaJson;

import com.fasterxml.jackson.databind.node.ObjectNode;
import com.sun.jna.Pointer;

public final class ActiveMssqlPlaybook implements AutoCloseable {
  private final String dependencyIdentifier;
  private Pointer handle;

  ActiveMssqlPlaybook(String dependencyIdentifier) {
    this.dependencyIdentifier = dependencyIdentifier;
  }

  public void begin(OpenArena arena) {
    ObjectNode spec = ArenaJson.object();
    spec.put("dependency_identifier", dependencyIdentifier);
    try {
      handle =
          ArenaFfiPlaybooks.mssqlPlaybookBegin(
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
    ArenaFfiPlaybooks.mssqlPlaybookFinish(h);
  }

  public void verify(String query, int expectedValue) {
    if (handle == null || Pointer.nativeValue(handle) == 0) {
      throw new IllegalStateException(
          "ActiveMssqlPlaybook.verify requires begun playbook scope (begin first)");
    }
    ObjectNode spec = ArenaJson.object();
    spec.put("dependency_identifier", dependencyIdentifier);
    spec.put("query", query);
    spec.put("expected_value", expectedValue);
    try {
      ArenaFfiPlaybooks.mssqlPlaybookVerify(handle, ArenaJson.MAPPER.writeValueAsString(spec));
    } catch (Exception e) {
      throw new ArenaBindingError(e.getMessage());
    }
  }
}
