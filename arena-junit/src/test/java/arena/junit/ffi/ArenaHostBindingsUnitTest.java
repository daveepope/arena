package arena.junit.ffi;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.io.IOException;
import java.net.InetSocketAddress;
import java.net.ServerSocket;
import java.util.ArrayList;
import java.util.List;

import org.junit.jupiter.api.Assumptions;
import org.junit.jupiter.api.Test;

final class ArenaHostBindingsUnitTest {

  @Test
  void findAvailablePort_randomStrategy_returnsPortWithinRange() {
    Assumptions.assumeTrue(ArenaNativeHolder.LIB != null);
    int port = ArenaBindings.findAvailablePort(23000, 23100, PortSearchStrategy.RANDOM);
    assertTrue(port >= 23000 && port < 23100);
  }

  @Test
  void findAvailablePort_linearStrategy_returnsPortWithinRange() {
    Assumptions.assumeTrue(ArenaNativeHolder.LIB != null);
    int port = ArenaBindings.findAvailablePort(23200, 23300, PortSearchStrategy.LINEAR);
    assertTrue(port >= 23200 && port < 23300);
  }

  @Test
  void findAvailablePort_exhaustedRange_throwsArenaPortNotFoundException() throws IOException {
    Assumptions.assumeTrue(ArenaNativeHolder.LIB != null);
    int rangeStart = 23400;
    int rangeEnd = 23402;
    List<ServerSocket> held = new ArrayList<>();
    try {
      for (int p = rangeStart; p < rangeEnd; p++) {
        ServerSocket socket = new ServerSocket();
        socket.bind(new InetSocketAddress("127.0.0.1", p));
        held.add(socket);
      }
      ArenaPortNotFoundException error =
          assertThrows(
              ArenaPortNotFoundException.class,
              () -> ArenaBindings.findAvailablePort(rangeStart, rangeEnd, PortSearchStrategy.LINEAR));
      assertEquals(ArenaStatus.PANIC, error.status());
    } finally {
      for (ServerSocket socket : held) {
        socket.close();
      }
    }
  }

  @Test
  void findAvailablePort_invertedRange_throwsArenaBindingErrorNotPortNotFound() {
    Assumptions.assumeTrue(ArenaNativeHolder.LIB != null);
    ArenaBindingError error =
        assertThrows(
            ArenaBindingError.class,
            () -> ArenaBindings.findAvailablePort(500, 500, PortSearchStrategy.RANDOM));
    assertTrue(error.status() == null || error.status() != ArenaStatus.PANIC);
  }
}
