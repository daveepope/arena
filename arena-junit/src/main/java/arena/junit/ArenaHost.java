package arena.junit;

import arena.junit.ffi.ArenaBindings;
import arena.junit.ffi.PortSearchStrategy;

public final class ArenaHost {
  private ArenaHost() {}

  public static int findAvailablePort(int rangeStart, int rangeEnd, PortSearchStrategy strategy) {
    return ArenaBindings.findAvailablePort(rangeStart, rangeEnd, strategy);
  }

  public static int findAvailablePort(int rangeStart, int rangeEnd) {
    return findAvailablePort(rangeStart, rangeEnd, PortSearchStrategy.RANDOM);
  }
}
