package arena.junit.ffi;

import static org.junit.jupiter.api.Assertions.assertDoesNotThrow;
import static org.junit.jupiter.api.Assertions.assertThrows;

import com.sun.jna.Pointer;

import org.junit.jupiter.api.Test;

final class ArenaBindingsUnitTest {

  @Test
  void arenaClose_nullHandle_returnsWithoutThrowing() {
    assertDoesNotThrow(() -> ArenaBindings.arenaClose(null));
  }

  @Test
  void arenaClose_zeroValueHandle_returnsWithoutThrowing() {
    assertDoesNotThrow(() -> ArenaBindings.arenaClose(Pointer.NULL));
  }

  @Test
  void activePlaybookDrop_nullHandle_returnsWithoutThrowing() {
    assertDoesNotThrow(() -> ArenaBindings.activePlaybookDrop(null));
  }

  @Test
  void activePlaybookDrop_zeroValueHandle_returnsWithoutThrowing() {
    assertDoesNotThrow(() -> ArenaBindings.activePlaybookDrop(Pointer.NULL));
  }

  @Test
  void registerDispatcherLoggingTarget_nullCallback_throwsArenaBindingError() {
    assertThrows(
        ArenaBindingError.class,
        () -> ArenaBindings.registerDispatcherLoggingTarget(null, Pointer.NULL));
  }

  @Test
  void unregisterDispatcherLoggingTarget_zeroToken_returnsWithoutThrowing() {
    assertDoesNotThrow(() -> ArenaBindings.unregisterDispatcherLoggingTarget(0L));
  }
}
