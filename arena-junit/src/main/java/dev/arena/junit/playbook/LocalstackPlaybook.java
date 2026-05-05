package dev.arena.junit.playbook;
import dev.arena.junit.OpenArena;
import dev.arena.junit.ffi.ArenaBindingError;
import dev.arena.junit.support.ArenaJson;

import com.fasterxml.jackson.databind.node.ObjectNode;
import com.sun.jna.Pointer;

public final class LocalstackPlaybook implements AutoCloseable {
  private final String dependencyIdentifier;
  private Pointer handle;

  LocalstackPlaybook(String dependencyIdentifier) {
    this.dependencyIdentifier = dependencyIdentifier;
  }

  public void open(OpenArena arena) {
    ObjectNode spec = ArenaJson.object();
    spec.put("dependency_identifier", dependencyIdentifier);
    try {
      handle =
          ArenaFfiPlaybooks.localstackPlaybookOpen(arena, ArenaJson.MAPPER.writeValueAsString(spec));
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
    ArenaFfiPlaybooks.localstackPlaybookClose(h);
  }
}
