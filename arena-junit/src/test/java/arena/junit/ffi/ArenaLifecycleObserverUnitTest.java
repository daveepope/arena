package arena.junit.ffi;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNotEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import arena.junit.lifecycle.ArenaState;
import com.sun.jna.Pointer;
import java.util.List;
import java.util.concurrent.CopyOnWriteArrayList;
import org.junit.jupiter.api.Test;

class ArenaLifecycleObserverUnitTest {

  private static void openAndClosePlainArena(String name) {
    Pointer handle = ArenaBindings.arenaOpen(name, null, ArenaLogLevel.INFO);
    ArenaBindings.arenaClose(handle);
  }

  private static List<String> statesOf(List<String> documents) {
    return documents.stream().map(document -> ArenaState.parse(document).state).toList();
  }

  @Test
  void addLifecycleObserverOpenAndCloseReportsInOrder() {
    List<String> documents = new CopyOnWriteArrayList<>();
    long token = ArenaBindings.addLifecycleObserver(documents::add);
    try {
      openAndClosePlainArena("junit-observer-order");
    } finally {
      ArenaBindings.removeLifecycleObserver(token);
    }

    List<String> states = statesOf(documents);
    assertFalse(states.isEmpty(), "no transitions were observed");
    assertEquals("arena_starting", states.get(0));
    assertEquals("arena_closed", states.get(states.size() - 1));
    assertTrue(states.indexOf("arena_open") < states.indexOf("arena_closing"));
  }

  @Test
  void addLifecycleObserverReportsArenaIdentifier() {
    List<String> documents = new CopyOnWriteArrayList<>();
    long token = ArenaBindings.addLifecycleObserver(documents::add);
    try {
      openAndClosePlainArena("junit-observer-identity");
    } finally {
      ArenaBindings.removeLifecycleObserver(token);
    }

    assertEquals("junit-observer-identity", ArenaState.parse(documents.get(0)).id);
  }

  @Test
  void removeLifecycleObserverBeforeOpenReportsNothing() {
    List<String> documents = new CopyOnWriteArrayList<>();
    long token = ArenaBindings.addLifecycleObserver(documents::add);
    ArenaBindings.removeLifecycleObserver(token);

    openAndClosePlainArena("junit-observer-removed");

    assertTrue(documents.isEmpty());
  }

  @Test
  void removeLifecycleObserverZeroTokenKeepsRegisteredObservers() {
    List<String> documents = new CopyOnWriteArrayList<>();
    long token = ArenaBindings.addLifecycleObserver(documents::add);
    try {
      ArenaBindings.removeLifecycleObserver(0L);
      openAndClosePlainArena("junit-observer-zero-token");
    } finally {
      ArenaBindings.removeLifecycleObserver(token);
    }

    assertFalse(documents.isEmpty(), "a zero token must not remove a registered observer");
  }

  @Test
  void arenaStateJsonClosedHandleThrowsBindingError() {
    org.junit.jupiter.api.Assertions.assertThrows(
        ArenaBindingError.class, () -> ArenaBindings.arenaStateJson(null));
  }

  @Test
  void addLifecycleObserverNullConsumerThrowsBindingError() {
    org.junit.jupiter.api.Assertions.assertThrows(
        ArenaBindingError.class, () -> ArenaBindings.addLifecycleObserver(null));
  }

  @Test
  void removeLifecycleObserverUnknownTokenIsIgnored() {
    ArenaBindings.removeLifecycleObserver(987654321L);
  }

  @Test
  void arenaStateJsonLiveArenaReturnsStateDocument() {
    Pointer handle = ArenaBindings.arenaOpen("junit-state-live", null, ArenaLogLevel.INFO);
    try {
      ArenaState state = ArenaState.parse(ArenaBindings.arenaStateJson(handle));
      assertEquals("junit-state-live", state.id);
      assertEquals("arena_open", state.state);
    } finally {
      ArenaBindings.arenaClose(handle);
    }
  }

  @Test
  void arenaCloseOpenArenaReturnsTerminalStateDocument() {
    Pointer handle = ArenaBindings.arenaOpen("junit-close-terminal", null, ArenaLogLevel.INFO);

    String document = ArenaBindings.arenaClose(handle);

    ArenaState state = ArenaState.parse(document);
    assertEquals("junit-close-terminal", state.id);
    assertEquals("arena_closed", state.state);
    assertNotEquals("arena_faulted", state.state);
  }
}
