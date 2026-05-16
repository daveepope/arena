package arena.junit.playbook;

import arena.junit.OpenArena;
import arena.junit.ffi.ArenaBindings;
import com.sun.jna.Pointer;

final class ArenaFfiPlaybooks {
  private ArenaFfiPlaybooks() {}

  public static Pointer httpPlaybookBegin(OpenArena arena, String specJson) {
    return ArenaBindings.httpPlaybookBegin(arena.handle(), specJson);
  }

  public static void httpPlaybookFinish(Pointer pb) {
    ArenaBindings.httpPlaybookFinish(pb);
  }

  public static void httpPlaybookVerify(Pointer pb, String specJson) {
    ArenaBindings.httpPlaybookVerify(pb, specJson);
  }

  public static Pointer mssqlPlaybookBegin(OpenArena arena, String specJson) {
    return ArenaBindings.mssqlPlaybookBegin(arena.handle(), specJson);
  }

  public static void mssqlPlaybookFinish(Pointer pb) {
    ArenaBindings.mssqlPlaybookFinish(pb);
  }

  public static void mssqlPlaybookVerify(Pointer pb, String specJson) {
    ArenaBindings.mssqlPlaybookVerify(pb, specJson);
  }

  public static Pointer localstackPlaybookBegin(OpenArena arena, String specJson) {
    return ArenaBindings.localstackPlaybookBegin(arena.handle(), specJson);
  }

  public static void localstackPlaybookFinish(Pointer pb) {
    ArenaBindings.localstackPlaybookFinish(pb);
  }
}
