package arena.junit.ffi;

public enum PortSearchStrategy {
  RANDOM(0),
  LINEAR(1);

  private final int code;

  PortSearchStrategy(int code) {
    this.code = code;
  }

  public int code() {
    return code;
  }
}
