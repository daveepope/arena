package arena.junit;

import static org.junit.jupiter.api.Assertions.assertDoesNotThrow;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

import arena.junit.match.ArenaRunnableComponent;
import arena.junit.match.ArenaRunnableDependency;
import arena.junit.match.Match;
import arena.junit.match.MatchBuilder;
import arena.junit.playbook.ActivePlaybook;
import arena.junit.playbook.ManagedPlaybook;
import arena.junit.playbook.PlaybookRegistration;
import arena.junit.support.ArenaJson;

import com.fasterxml.jackson.databind.node.ObjectNode;
import com.sun.jna.Pointer;

import java.util.List;
import org.junit.jupiter.api.Test;

final class OpenArenaUnitTest {

  static final class StubMatchPiece implements ArenaRunnableDependency, ArenaRunnableComponent {
    @Override
    public ObjectNode forFfi() {
      return ArenaJson.object();
    }
  }

  static final class StubRegisteredPlaybook implements ManagedPlaybook, PlaybookRegistration {
    @Override
    public String identifier() {
      return "stub-registered";
    }

    @Override
    public ActivePlaybook run(OpenArena arena) {
      throw new UnsupportedOperationException("not used in this test");
    }

    @Override
    public ObjectNode forRegisteredFfi() {
      return ArenaJson.object().put("identifier", identifier());
    }
  }

  static final class UnregisteredPlaybook implements ManagedPlaybook {
    @Override
    public String identifier() {
      return "unregistered";
    }

    @Override
    public ActivePlaybook run(OpenArena arena) {
      throw new UnsupportedOperationException("not used in this test");
    }
  }

  private static Match matchWithRegisteredPlaybook() {
    return new MatchBuilder("match")
        .addDependency(new StubMatchPiece())
        .registerPlaybook(new StubRegisteredPlaybook(), true)
        .build();
  }

  @Test
  void handle_constructedWithPointer_returnsSamePointer() {
    OpenArena arena = new OpenArena(Pointer.NULL, 0L, List.of());
    assertEquals(Pointer.NULL, arena.handle());
  }

  @Test
  void matches_constructedWithList_returnsImmutableCopy() {
    Match match = matchWithRegisteredPlaybook();
    OpenArena arena = new OpenArena(Pointer.NULL, 0L, List.of(match));
    assertEquals(List.of(match), arena.matches());
  }

  @Test
  void playbook_registeredClass_returnsRegisteredInstance() {
    OpenArena arena = new OpenArena(Pointer.NULL, 0L, List.of(matchWithRegisteredPlaybook()));
    assertTrue(arena.playbook(StubRegisteredPlaybook.class) instanceof StubRegisteredPlaybook);
  }

  @Test
  void playbook_unregisteredClass_returnsNull() {
    OpenArena arena = new OpenArena(Pointer.NULL, 0L, List.of(matchWithRegisteredPlaybook()));
    assertNull(arena.playbook(UnregisteredPlaybook.class));
  }

  @Test
  void playbookExecOnDependencyStart_registeredClass_returnsConfiguredFlag() {
    OpenArena arena = new OpenArena(Pointer.NULL, 0L, List.of(matchWithRegisteredPlaybook()));
    assertTrue(arena.playbookExecOnDependencyStart(StubRegisteredPlaybook.class));
  }

  @Test
  void playbookExecOnDependencyStart_unregisteredClass_returnsNull() {
    OpenArena arena = new OpenArena(Pointer.NULL, 0L, List.of(matchWithRegisteredPlaybook()));
    assertNull(arena.playbookExecOnDependencyStart(UnregisteredPlaybook.class));
  }

  @Test
  void close_zeroHandleAndZeroToken_returnsWithoutThrowing() {
    OpenArena arena = new OpenArena(Pointer.NULL, 0L, List.of());
    assertDoesNotThrow(arena::close);
  }

  @Test
  void close_nullHandleAndZeroToken_returnsWithoutThrowing() {
    OpenArena arena = new OpenArena(null, 0L, List.of());
    assertDoesNotThrow(arena::close);
  }
}
