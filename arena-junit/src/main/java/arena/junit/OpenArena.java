package arena.junit;

import com.sun.jna.Pointer;
import arena.junit.ffi.ArenaBindings;
import arena.junit.ffi.ArenaStatus;

public final class OpenArena {
  private final Pointer handle;

  OpenArena(Pointer handle) {
    this.handle = handle;
  }

  public Pointer handle() {
    return handle;
  }

  public void close() {
    ArenaBindings.arenaClose(handle);
  }

  public ArenaStatus softReset(String dependencyIdentifier) {
    return ArenaBindings.softReset(handle, dependencyIdentifier);
  }

  public ArenaStatus hardReset(String dependencyIdentifier) {
    return ArenaBindings.hardReset(handle, dependencyIdentifier);
  }
}
