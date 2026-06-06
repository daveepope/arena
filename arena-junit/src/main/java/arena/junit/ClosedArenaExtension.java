package arena.junit;

import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.atomic.AtomicBoolean;
import org.junit.jupiter.api.extension.AfterAllCallback;
import org.junit.jupiter.api.extension.BeforeAllCallback;
import org.junit.jupiter.api.extension.ExtensionContext;

public abstract class ClosedArenaExtension implements BeforeAllCallback, AfterAllCallback {

  private static final AtomicBoolean SHUTDOWN_HOOK_REGISTERED = new AtomicBoolean(false);
  private static final ConcurrentHashMap<ClosedArenaExtension, OpenArena> SHUTDOWN_ARENAS =
      new ConcurrentHashMap<>();

  private volatile OpenArena openArena;
  private int refs;

  protected abstract ClosedArena buildClosedArena() throws Exception;

  protected void afterOpen(OpenArena openArena) throws Exception {}

  protected void beforeClose(OpenArena openArena) {}

  public OpenArena openArena() {
    OpenArena arena = openArena;
    if (arena == null) {
      throw new IllegalStateException(
          "ClosedArenaExtension: open arena is not available (extension not initialized)");
    }
    return arena;
  }

  private static void registerShutdownHookOnce() {
    if (!SHUTDOWN_HOOK_REGISTERED.compareAndSet(false, true)) {
      return;
    }
    Runtime.getRuntime()
        .addShutdownHook(
            new Thread(
                () -> {
                  for (OpenArena arena : SHUTDOWN_ARENAS.values()) {
                    if (arena != null) {
                      arena.close();
                    }
                  }
                  SHUTDOWN_ARENAS.clear();
                },
                "arena-junit-closed-arena-shutdown"));
  }

  @Override
  public void beforeAll(ExtensionContext context) throws Exception {
    synchronized (this) {
      if (openArena == null) {
        openArena = buildClosedArena().open();
        afterOpen(openArena);
        registerShutdownHookOnce();
        SHUTDOWN_ARENAS.put(this, openArena);
      }
      refs++;
    }
  }

  @Override
  public void afterAll(ExtensionContext context) {
    synchronized (this) {
      refs--;
      if (refs == 0 && openArena != null) {
        beforeClose(openArena);
        openArena.close();
        SHUTDOWN_ARENAS.remove(this);
        openArena = null;
      }
    }
  }
}
