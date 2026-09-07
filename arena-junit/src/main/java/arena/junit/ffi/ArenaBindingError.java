package arena.junit.ffi;
public class ArenaBindingError extends RuntimeException {
  private final ArenaStatus status;
  private final String stateDocument;

  public ArenaBindingError(String message) {
    super(message);
    this.status = null;
    this.stateDocument = null;
  }

  public ArenaBindingError(String message, ArenaStatus status) {
    super(message);
    this.status = status;
    this.stateDocument = null;
  }

  public ArenaBindingError(String message, ArenaStatus status, String stateDocument) {
    super(message);
    this.status = status;
    this.stateDocument = stateDocument;
  }

  public ArenaStatus status() {
    return status;
  }

  public String stateDocument() {
    return stateDocument;
  }
}
