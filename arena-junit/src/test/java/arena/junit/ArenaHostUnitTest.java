package arena.junit;

import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import arena.junit.ffi.ArenaBindingError;
import arena.junit.ffi.PortSearchStrategy;
import org.junit.jupiter.api.Test;

final class ArenaHostUnitTest {

  @Test
  void findAvailablePort_defaultStrategyOverload_returnsPortWithinRange() {
    int port = ArenaHost.findAvailablePort(23800, 23900);
    assertTrue(port >= 23800 && port < 23900);
  }

  @Test
  void findAvailablePort_explicitStrategyOverload_returnsPortWithinRange() {
    int port = ArenaHost.findAvailablePort(23900, 24000, PortSearchStrategy.LINEAR);
    assertTrue(port >= 23900 && port < 24000);
  }

  @Test
  void findAvailablePort_invertedRange_throwsArenaBindingError() {
    assertThrows(ArenaBindingError.class, () -> ArenaHost.findAvailablePort(500, 500));
  }
}
