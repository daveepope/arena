package arena.junit.playbook;

import arena.junit.ffi.ArenaBindings;
import arena.junit.ffi.ArenaBindingError;

import com.sun.jna.Pointer;

public abstract class ActivePlaybook implements AutoCloseable {
  private Pointer handle;
  private boolean bodyFailed;

  protected ActivePlaybook(Pointer handle) {
    if (handle == null || Pointer.nativeValue(handle) == 0) {
      throw new IllegalArgumentException("ActivePlaybook requires a non-null native handle");
    }
    this.handle = handle;
  }

  protected final Pointer handle() {
    Pointer h = handle;
    if (h == null || Pointer.nativeValue(h) == 0) {
      throw new IllegalStateException("active playbook is already closed");
    }
    return h;
  }

  protected final void noteBodyFailure() {
    bodyFailed = true;
  }

  @Override
  public final void close() {
    Pointer h = handle;
    handle = null;
    if (h == null || Pointer.nativeValue(h) == 0) {
      return;
    }
    try {
      ArenaBindings.activePlaybookDrop(h);
    } catch (ArenaBindingError e) {
      if (bodyFailed) {
        return;
      }
      throw e;
    }
  }
}
