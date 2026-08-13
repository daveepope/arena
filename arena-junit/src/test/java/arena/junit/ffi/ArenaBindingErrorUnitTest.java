package arena.junit.ffi;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNull;

import org.junit.jupiter.api.Test;

final class ArenaBindingErrorUnitTest {

  @Test
  void messageOnly_constructor_leavesStatusNull() {
    ArenaBindingError error = new ArenaBindingError("boom");
    assertEquals("boom", error.getMessage());
    assertNull(error.status());
  }

  @Test
  void messageAndStatus_constructor_carriesBothFields() {
    ArenaBindingError error = new ArenaBindingError("boom", ArenaStatus.FAILED);
    assertEquals("boom", error.getMessage());
    assertEquals(ArenaStatus.FAILED, error.status());
  }
}
