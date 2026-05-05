package arena.junit.ffi;
public enum ArenaStatus {
  OK(0),
  INVALID_ARGUMENT(1),
  FAILED(2),
  PANIC(3),
  NOT_FOUND(4);

  private final int code;

  ArenaStatus(int code) {
    this.code = code;
  }

  public int code() {
    return code;
  }

  public static ArenaStatus fromInt(int raw) {
    for (ArenaStatus s : values()) {
      if (s.code == raw) {
        return s;
      }
    }
    throw new IllegalArgumentException("unknown arena status code: " + raw);
  }
}
