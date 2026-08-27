package arena.junit.ffi;
public class ArenaBindingError extends RuntimeException {
  private final ArenaStatus status;

  public ArenaBindingError(String message) {
    super(message);
    this.status = null;
  }

  public ArenaBindingError(String message, ArenaStatus status) {
    super(message);
    this.status = status;
  }

  public ArenaStatus status() {
    return status;
  }
}
