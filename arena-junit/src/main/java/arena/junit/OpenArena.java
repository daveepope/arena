package arena.junit;

import com.sun.jna.Pointer;
import arena.junit.ffi.ArenaBindings;
import arena.junit.ffi.ArenaLogbackFlush;
import arena.junit.ffi.ArenaStatus;

public final class OpenArena {
  private Pointer handle;
  private volatile long dispatcherLoggingTargetToken;

  OpenArena(Pointer handle, long dispatcherLoggingTargetToken) {
    this.handle = handle;
    this.dispatcherLoggingTargetToken = dispatcherLoggingTargetToken;
  }

  public Pointer handle() {
    return handle;
  }

  public void close() {
    Pointer h = handle;
    if (h == null || Pointer.nativeValue(h) == 0) {
      long t = dispatcherLoggingTargetToken;
      if (t != 0L) {
        ArenaBindings.unregisterDispatcherLoggingTarget(t);
        dispatcherLoggingTargetToken = 0L;
      }
      return;
    }
    handle = null;
    try {
      ArenaBindings.arenaClose(h);
    } finally {
      ArenaLogbackFlush.flushIfPresent();
      long tok = dispatcherLoggingTargetToken;
      if (tok != 0L) {
        ArenaBindings.unregisterDispatcherLoggingTarget(tok);
        dispatcherLoggingTargetToken = 0L;
      }
    }
  }

  public ArenaStatus softReset(String dependencyIdentifier) {
    return ArenaBindings.softReset(handle, dependencyIdentifier);
  }

  public ArenaStatus hardReset(String dependencyIdentifier) {
    return ArenaBindings.hardReset(handle, dependencyIdentifier);
  }
}
