package dev.arena.junit.playbook;

import com.sun.jna.Pointer;
import dev.arena.junit.OpenArena;
import dev.arena.junit.ffi.ArenaBindings;

public final class ArenaFfiPlaybooks {
  private ArenaFfiPlaybooks() {}

  public static Pointer httpPlaybookOpen(OpenArena arena, String specJson) {
    return ArenaBindings.httpPlaybookOpen(arena.handle(), specJson);
  }

  public static void httpPlaybookClose(Pointer pb) {
    ArenaBindings.httpPlaybookClose(pb);
  }

  public static void httpPlaybookVerify(Pointer pb, String specJson) {
    ArenaBindings.httpPlaybookVerify(pb, specJson);
  }

  public static Pointer mssqlPlaybookOpen(OpenArena arena, String specJson) {
    return ArenaBindings.mssqlPlaybookOpen(arena.handle(), specJson);
  }

  public static void mssqlPlaybookClose(Pointer pb) {
    ArenaBindings.mssqlPlaybookClose(pb);
  }

  public static void mssqlPlaybookVerify(Pointer pb, String specJson) {
    ArenaBindings.mssqlPlaybookVerify(pb, specJson);
  }

  public static Pointer localstackPlaybookOpen(OpenArena arena, String specJson) {
    return ArenaBindings.localstackPlaybookOpen(arena.handle(), specJson);
  }

  public static void localstackPlaybookClose(Pointer pb) {
    ArenaBindings.localstackPlaybookClose(pb);
  }
}
