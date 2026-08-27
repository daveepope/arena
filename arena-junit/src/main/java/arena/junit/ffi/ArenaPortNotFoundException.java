package arena.junit.ffi;

public final class ArenaPortNotFoundException extends ArenaBindingError {
  public ArenaPortNotFoundException(String message) {
    super(message, ArenaStatus.PANIC);
  }
}
