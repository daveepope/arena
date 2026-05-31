package arena.junit;

import org.junit.jupiter.api.extension.AfterAllCallback;
import org.junit.jupiter.api.extension.BeforeAllCallback;
import org.junit.jupiter.api.extension.ExtensionContext;

public abstract class ClosedArenaExtension implements BeforeAllCallback, AfterAllCallback {

  private static final ExtensionContext.Namespace NS =
      ExtensionContext.Namespace.create("arena.junit.closed");

  private static volatile OpenArena CURRENT;

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
      }
      h.refs++;
      CURRENT = h.openArena;
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
      }
    }
  }

  private static final class Holder {
    OpenArena openArena;
    int refs;
  }
}
