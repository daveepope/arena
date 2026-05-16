package arena.junit.ffi;

public enum ArenaLogLevel {
  ERROR(1),
  WARN(2),
  INFO(3),
  DEBUG(4),
  TRACE(5);

  private final int code;

  ArenaLogLevel(int code) {
    this.code = code;
  }

  public int code() {
    return code;
  }
}
