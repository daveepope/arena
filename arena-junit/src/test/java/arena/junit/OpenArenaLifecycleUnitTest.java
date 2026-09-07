package arena.junit;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import arena.junit.exec.ExecutableComponentBuilder;
import arena.junit.ffi.ArenaLogLevel;
import arena.junit.lifecycle.ArenaLifecycleError;
import arena.junit.lifecycle.ComponentState;
import arena.junit.match.Match;
import arena.junit.match.MatchBuilder;
import java.util.List;
import org.junit.jupiter.api.Test;

class OpenArenaLifecycleUnitTest {

  private static ClosedArena closedArenaWithMissingBinary(String arenaName) {
    Match match =
        new MatchBuilder(arenaName + "-match")
            .addComponent(
                new ExecutableComponentBuilder("junit-lifecycle-missing-binary")
                    .withExecutablePath("/nonexistent/junit-lifecycle-probe")
                    .build())
            .build();
    return new ClosedArena(arenaName, List.of(match), ArenaLogLevel.INFO);
  }

  @Test
  void openFaultedComponentRaisesLifecycleErrorWithState() {
    ClosedArena closed = closedArenaWithMissingBinary("junit-lifecycle-faulted");

    ArenaLifecycleError error = assertThrows(ArenaLifecycleError.class, closed::open);

    assertTrue(error.getMessage().contains("is arena_faulted"));
    assertNotNull(error.state());
    assertTrue(error.state().isFaulted());
    assertEquals("junit-lifecycle-faulted", error.state().id);
    ComponentState component =
        error.state().components.stream()
            .filter(c -> c.id.contains("junit-lifecycle-missing-binary"))
            .findFirst()
            .orElse(null);
    assertNotNull(component);
    assertFalse(error.getMessage().contains("panicked at"));
  }

  @Test
  void stateOpenArenaReturnsOpenState() {
    Match plain = new MatchBuilder("junit-state-accessor-match").build();
    ClosedArena closed =
        new ClosedArena("junit-state-accessor", List.of(plain), ArenaLogLevel.INFO);
    OpenArena open;
    try {
      open = closed.open();
    } catch (Exception e) {
      throw new AssertionError(e);
    }
    try {
      assertEquals("junit-state-accessor", open.state().id);
      assertEquals("arena_open", open.state().state);
    } finally {
      open.close();
    }
  }
}
