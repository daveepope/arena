package arena.junit.ffi;
import com.sun.jna.Pointer;
import com.sun.jna.ptr.PointerByReference;
import java.nio.charset.StandardCharsets;

public final class ArenaBindings {
  private ArenaBindings() {}

  public static ArenaNativeLib lib() {
    ArenaNativeLib lib = ArenaNativeHolder.LIB;
    if (lib == null) {
      throw new ArenaBindingError("arena shared library not found (set ARENA_FFI_LIB or use Bazel runfiles)");
    }
    return lib;
  }

  public static String takeErr(PointerByReference errSlot) {
    Pointer p = errSlot.getValue();
    if (p == null) {
      return null;
    }
    try {
      long peer = Pointer.nativeValue(p);
      if (peer == 0) {
        return null;
      }
      return p.getString(0, StandardCharsets.UTF_8.name());
    } finally {
      ArenaNativeHolder.LIB.arena_free_string(p);
      errSlot.setValue(null);
    }
  }

  public static Pointer arenaOpen(String name, String configJson) {
    ArenaNativeLib lib = lib();
    PointerByReference err = new PointerByReference();
    Pointer h = lib.arena_open(name, configJson, err);
    if (h == null || Pointer.nativeValue(h) == 0) {
      String msg = takeErr(err);
      throw new ArenaBindingError(msg != null ? msg : "arena_open returned null");
    }
    return h;
  }

  public static void arenaClose(Pointer handle) {
    if (handle == null || Pointer.nativeValue(handle) == 0) {
      return;
    }
    ArenaNativeHolder.LIB.arena_close(handle);
  }

  public static ArenaStatus softReset(Pointer arena, String dependencyIdentifier) {
    return reset(arena, dependencyIdentifier, true);
  }

  public static ArenaStatus hardReset(Pointer arena, String dependencyIdentifier) {
    return reset(arena, dependencyIdentifier, false);
  }

  private static ArenaStatus reset(Pointer arena, String dependencyIdentifier, boolean soft) {
    PointerByReference err = new PointerByReference();
    int raw =
        soft
            ? ArenaNativeHolder.LIB.arena_soft_reset(arena, dependencyIdentifier, err)
            : ArenaNativeHolder.LIB.arena_hard_reset(arena, dependencyIdentifier, err);
    String msg = takeErr(err);
    ArenaStatus st;
    try {
      st = ArenaStatus.fromInt(raw);
    } catch (IllegalArgumentException e) {
      throw new ArenaBindingError(msg != null ? msg : "reset returned unknown status " + raw);
    }
    if (st != ArenaStatus.OK) {
      throw new ArenaBindingError(msg != null ? msg : "reset failed: " + st, st);
    }
    return st;
  }

  public static String oauthLoopbackTlsPemJson() {
    ArenaNativeLib lib = lib();
    PointerByReference err = new PointerByReference();
    Pointer raw = lib.arena_oauth_loopback_tls_pem_json(err);
    if (raw == null || Pointer.nativeValue(raw) == 0) {
      String msg = takeErr(err);
      throw new ArenaBindingError(msg != null ? msg : "arena_oauth_loopback_tls_pem_json returned null");
    }
    try {
      return raw.getString(0, StandardCharsets.UTF_8.name());
    } finally {
      lib.arena_free_string(raw);
    }
  }

  public static Pointer httpPlaybookOpen(Pointer arena, String specJson) {
    ArenaNativeLib lib = lib();
    PointerByReference err = new PointerByReference();
    Pointer pb = lib.arena_http_playbook_open(arena, specJson, err);
    if (pb == null || Pointer.nativeValue(pb) == 0) {
      String msg = takeErr(err);
      throw new ArenaBindingError(msg != null ? msg : "arena_http_playbook_open returned null");
    }
    return pb;
  }

  public static void httpPlaybookClose(Pointer pb) {
    if (pb == null || Pointer.nativeValue(pb) == 0) {
      return;
    }
    PointerByReference err = new PointerByReference();
    int raw = lib().arena_http_playbook_close(pb, err);
    String msg = takeErr(err);
    ArenaStatus st;
    try {
      st = ArenaStatus.fromInt(raw);
    } catch (IllegalArgumentException e) {
      throw new ArenaBindingError(msg != null ? msg : "http_playbook_close unknown status " + raw);
    }
    if (st != ArenaStatus.OK) {
      throw new ArenaBindingError(msg != null ? msg : "http_playbook_close failed: " + st, st);
    }
  }

  public static void httpPlaybookVerify(Pointer pb, String specJson) {
    PointerByReference err = new PointerByReference();
    int raw = lib().arena_http_playbook_verify(pb, specJson, err);
    String msg = takeErr(err);
    ArenaStatus st;
    try {
      st = ArenaStatus.fromInt(raw);
    } catch (IllegalArgumentException e) {
      throw new ArenaBindingError(msg != null ? msg : "http_playbook_verify unknown status " + raw);
    }
    if (st != ArenaStatus.OK) {
      throw new ArenaBindingError(msg != null ? msg : "http_playbook_verify failed: " + st, st);
    }
  }

  public static Pointer mssqlPlaybookOpen(Pointer arena, String specJson) {
    ArenaNativeLib lib = lib();
    PointerByReference err = new PointerByReference();
    Pointer pb = lib.arena_mssql_playbook_open(arena, specJson, err);
    if (pb == null || Pointer.nativeValue(pb) == 0) {
      String msg = takeErr(err);
      throw new ArenaBindingError(msg != null ? msg : "arena_mssql_playbook_open returned null");
    }
    return pb;
  }

  public static void mssqlPlaybookClose(Pointer pb) {
    if (pb == null || Pointer.nativeValue(pb) == 0) {
      return;
    }
    PointerByReference err = new PointerByReference();
    int raw = lib().arena_mssql_playbook_close(pb, err);
    String msg = takeErr(err);
    ArenaStatus st;
    try {
      st = ArenaStatus.fromInt(raw);
    } catch (IllegalArgumentException e) {
      throw new ArenaBindingError(msg != null ? msg : "mssql_playbook_close unknown status " + raw);
    }
    if (st != ArenaStatus.OK) {
      throw new ArenaBindingError(msg != null ? msg : "mssql_playbook_close failed: " + st, st);
    }
  }

  public static void mssqlPlaybookVerify(Pointer pb, String specJson) {
    PointerByReference err = new PointerByReference();
    int raw = lib().arena_mssql_playbook_verify(pb, specJson, err);
    String msg = takeErr(err);
    ArenaStatus st;
    try {
      st = ArenaStatus.fromInt(raw);
    } catch (IllegalArgumentException e) {
      throw new ArenaBindingError(msg != null ? msg : "mssql_playbook_verify unknown status " + raw);
    }
    if (st != ArenaStatus.OK) {
      throw new ArenaBindingError(msg != null ? msg : "mssql_playbook_verify failed: " + st, st);
    }
  }

  public static Pointer localstackPlaybookOpen(Pointer arena, String specJson) {
    ArenaNativeLib lib = lib();
    PointerByReference err = new PointerByReference();
    Pointer pb = lib.arena_localstack_playbook_open(arena, specJson, err);
    if (pb == null || Pointer.nativeValue(pb) == 0) {
      String msg = takeErr(err);
      throw new ArenaBindingError(msg != null ? msg : "arena_localstack_playbook_open returned null");
    }
    return pb;
  }

  public static void localstackPlaybookClose(Pointer pb) {
    if (pb == null || Pointer.nativeValue(pb) == 0) {
      return;
    }
    PointerByReference err = new PointerByReference();
    int raw = lib().arena_localstack_playbook_close(pb, err);
    String msg = takeErr(err);
    ArenaStatus st;
    try {
      st = ArenaStatus.fromInt(raw);
    } catch (IllegalArgumentException e) {
      throw new ArenaBindingError(msg != null ? msg : "localstack_playbook_close unknown status " + raw);
    }
    if (st != ArenaStatus.OK) {
      throw new ArenaBindingError(msg != null ? msg : "localstack_playbook_close failed: " + st, st);
    }
  }
}
