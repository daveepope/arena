package arena.junit;

import java.util.concurrent.atomic.AtomicBoolean;
import org.junit.jupiter.api.extension.AfterAllCallback;
import org.junit.jupiter.api.extension.BeforeAllCallback;
import org.junit.jupiter.api.extension.ExtensionContext;

public abstract class ClosedArenaExtension implements BeforeAllCallback, AfterAllCallback {

  private static final ExtensionContext.Namespace NS =
      ExtensionContext.Namespace.create("arena.junit.closed");

  private static volatile OpenArena CURRENT;
  private static volatile OpenArena SHUTDOWN_HOOK_ARENA;
  private static final AtomicBoolean SHUTDOWN_HOOK_REGISTERED = new AtomicBoolean(false);

  protected abstract ClosedArena buildClosedArena() throws Exception;

  protected void afterOpen(OpenArena openArena) throws Exception {}

  protected void beforeClose(OpenArena openArena) {}

  public OpenArena openArena() {
    OpenArena a = CURRENT;
    if (a == null) {
      throw new IllegalStateException(
          "ClosedArenaExtension: open arena is not available (extension not initialized)");
    }
    return a;
  }

  private static void registerShutdownHookOnce() {
    if (!SHUTDOWN_HOOK_REGISTERED.compareAndSet(false, true)) {
      return;
    }
    Runtime.getRuntime()
        .addShutdownHook(
            new Thread(
                () -> {
                  OpenArena arena = SHUTDOWN_HOOK_ARENA;
                  if (arena != null) {
                    arena.close();
                  }
                },
                "arena-junit-closed-arena-shutdown"));
  }

  @Override
  public void beforeAll(ExtensionContext context) throws Exception {
    ExtensionContext root = context.getRoot();
    ExtensionContext.Store store = root.getStore(NS);
    Holder h = store.get("holder", Holder.class);
    if (h == null) {
      h = new Holder();
      store.put("holder", h);
    }
    synchronized (h) {
      if (h.openArena == null) {
        h.openArena = buildClosedArena().open();
        afterOpen(h.openArena);
        registerShutdownHookOnce();
      }
      h.refs++;
      CURRENT = h.openArena;
      SHUTDOWN_HOOK_ARENA = h.openArena;
    }
  }

  @Override
  public void afterAll(ExtensionContext context) {
    ExtensionContext root = context.getRoot();
    ExtensionContext.Store store = root.getStore(NS);
    Holder h = store.get("holder", Holder.class);
    if (h == null) {
      return;
    }
    synchronized (h) {
      h.refs--;
      if (h.refs == 0 && h.openArena != null) {
        beforeClose(h.openArena);
        h.openArena.close();
        h.openArena = null;
        CURRENT = null;
        SHUTDOWN_HOOK_ARENA = null;
      }
    }
  }

  private static final class Holder {
    OpenArena openArena;
    int refs;
  }
}
