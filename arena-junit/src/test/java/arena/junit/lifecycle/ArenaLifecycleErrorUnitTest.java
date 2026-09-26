package arena.junit.lifecycle;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertInstanceOf;
import static org.junit.jupiter.api.Assertions.assertSame;

import arena.junit.ffi.ArenaBindingError;
import org.junit.jupiter.api.Test;

class ArenaLifecycleErrorUnitTest {

  @Test
  void fromStateDocumentReturnsLifecycleErrorWithState() {
    ArenaBindingError error =
        new ArenaBindingError("open failed", null, ArenaStateUnitTest.FIXTURE_STATE_JSON);

    ArenaBindingError converted = ArenaLifecycleError.from(error);

    ArenaLifecycleError lifecycle = assertInstanceOf(ArenaLifecycleError.class, converted);
    assertEquals("open failed", lifecycle.getMessage());
    assertEquals("orders", lifecycle.state().id);
  }

  @Test
  void fromNoStateDocumentReturnsOriginalError() {
    ArenaBindingError error = new ArenaBindingError("plain failure");

    assertSame(error, ArenaLifecycleError.from(error));
  }

  @Test
  void fromUnparseableDocumentReturnsOriginalError() {
    ArenaBindingError error = new ArenaBindingError("open failed", null, "{not json");

    assertSame(error, ArenaLifecycleError.from(error));
  }

  @Test
  void fromLifecycleErrorReturnsItUnchanged() {
    ArenaLifecycleError error = new ArenaLifecycleError("already converted", null);

    assertSame(error, ArenaLifecycleError.from(error));
  }
}
