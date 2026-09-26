package arena.junit.lifecycle;

import arena.junit.ffi.ArenaBindingError;

public class ArenaLifecycleError extends ArenaBindingError {
  private final transient ArenaState state;

  public ArenaLifecycleError(String message, ArenaState state) {
    super(message);
    this.state = state;
  }

  public ArenaState state() {
    return state;
  }

  public static ArenaBindingError from(ArenaBindingError error) {
    if (error instanceof ArenaLifecycleError) {
      return error;
    }
    String document = error.stateDocument();
    if (document == null || document.isEmpty()) {
      return error;
    }
    ArenaState state;
    try {
      state = ArenaState.parse(document);
    } catch (IllegalArgumentException e) {
      return error;
    }
    return new ArenaLifecycleError(error.getMessage(), state);
  }
}
