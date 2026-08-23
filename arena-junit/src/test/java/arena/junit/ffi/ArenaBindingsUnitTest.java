package arena.junit.ffi;

import static org.junit.jupiter.api.Assertions.assertDoesNotThrow;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNotEquals;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertThrows;

import com.sun.jna.Pointer;

import org.junit.jupiter.api.Assumptions;
import org.junit.jupiter.api.Test;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

final class ArenaBindingsUnitTest {

  private static Pointer openEmptyArena(String name) {
    return ArenaBindings.arenaOpen(name, "{}", ArenaLogLevel.INFO);
  }

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

  @Test
  void lib_nativeLibLoaded_returnsNonNullInstance() {
    Assumptions.assumeTrue(ArenaNativeHolder.LIB != null);
    assertNotNull(ArenaBindings.lib());
  }

  @Test
  void arenaOpen_emptyMatchConfig_returnsHandleThenCloses() {
    Assumptions.assumeTrue(ArenaNativeHolder.LIB != null);
    Pointer handle = openEmptyArena("arena-bindings-open-empty");
    try {
      assertNotEquals(0L, Pointer.nativeValue(handle));
    } finally {
      ArenaBindings.arenaClose(handle);
    }
  }

  @Test
  void arenaOpen_twoArgOverload_returnsHandleThenCloses() {
    Assumptions.assumeTrue(ArenaNativeHolder.LIB != null);
    Pointer handle = ArenaBindings.arenaOpen("arena-bindings-open-two-arg", "{}");
    try {
      assertNotEquals(0L, Pointer.nativeValue(handle));
    } finally {
      ArenaBindings.arenaClose(handle);
    }
  }

  @Test
  void arenaOpen_malformedConfigJson_throwsArenaBindingError() {
    Assumptions.assumeTrue(ArenaNativeHolder.LIB != null);
    assertThrows(
        ArenaBindingError.class,
        () -> ArenaBindings.arenaOpen("arena-bindings-open-malformed", "not valid json"));
  }

  @Test
  void oauthLoopbackTlsPemJson_validCall_returnsNonEmptyDocument() {
    Assumptions.assumeTrue(ArenaNativeHolder.LIB != null);
    String document = ArenaBindings.oauthLoopbackTlsPemJson();
    assertFalse(document.isEmpty());
  }

  @Test
  void softReset_unknownDependency_throwsArenaBindingErrorWithNotFoundStatus() {
    Assumptions.assumeTrue(ArenaNativeHolder.LIB != null);
    Pointer handle = openEmptyArena("arena-bindings-soft-reset-unknown");
    try {
      ArenaBindingError error =
          assertThrows(
              ArenaBindingError.class, () -> ArenaBindings.softReset(handle, "does-not-exist"));
      assertEquals(ArenaStatus.NOT_FOUND, error.status());
    } finally {
      ArenaBindings.arenaClose(handle);
    }
  }

  @Test
  void hardReset_unknownDependency_throwsArenaBindingErrorWithNotFoundStatus() {
    Assumptions.assumeTrue(ArenaNativeHolder.LIB != null);
    Pointer handle = openEmptyArena("arena-bindings-hard-reset-unknown");
    try {
      ArenaBindingError error =
          assertThrows(
              ArenaBindingError.class, () -> ArenaBindings.hardReset(handle, "does-not-exist"));
      assertEquals(ArenaStatus.NOT_FOUND, error.status());
    } finally {
      ArenaBindings.arenaClose(handle);
    }
  }

  @Test
  void matchPlaybookRun_unknownIdentifier_throwsArenaBindingError() {
    Assumptions.assumeTrue(ArenaNativeHolder.LIB != null);
    Pointer handle = openEmptyArena("arena-bindings-playbook-run-unknown");
    try {
      assertThrows(
          ArenaBindingError.class,
          () -> ArenaBindings.matchPlaybookRun(handle, "does-not-exist"));
    } finally {
      ArenaBindings.arenaClose(handle);
    }
  }

  @Test
  void registerDefaultDispatcherLoggingTarget_noArgOverload_returnsNonZeroTokenThenUnregisters() {
    Assumptions.assumeTrue(ArenaNativeHolder.LIB != null);
    long token = ArenaBindings.registerDefaultDispatcherLoggingTarget();
    try {
      assertNotEquals(0L, token);
    } finally {
      ArenaBindings.unregisterDispatcherLoggingTarget(token);
    }
  }

  @Test
  void registerSlf4jDispatcherLoggingTarget_singleArgOverload_returnsNonZeroTokenThenUnregisters() {
    Assumptions.assumeTrue(ArenaNativeHolder.LIB != null);
    Logger logger = LoggerFactory.getLogger("arena-bindings-slf4j-single-arg");
    long token = ArenaBindings.registerSlf4jDispatcherLoggingTarget(logger);
    try {
      assertNotEquals(0L, token);
    } finally {
      ArenaBindings.unregisterDispatcherLoggingTarget(token);
    }
  }
}
