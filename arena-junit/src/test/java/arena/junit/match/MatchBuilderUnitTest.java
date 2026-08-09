package arena.junit.match;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import arena.junit.OpenArena;
import arena.junit.playbook.ActivePlaybook;
import arena.junit.playbook.ManagedPlaybook;
import arena.junit.playbook.Playbook;
import arena.junit.playbook.PlaybookRegistration;
import arena.junit.playbook.UnmanagedPlaybook;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.node.ObjectNode;
import org.junit.jupiter.api.Test;

final class MatchBuilderUnitTest {

  private static final ObjectMapper MAPPER = new ObjectMapper();

  static final class StubMatchPiece implements ArenaRunnableDependency, ArenaRunnableComponent {
    @Override
    public ObjectNode forFfi() {
      return MAPPER.createObjectNode();
    }
  }

  static final class PlainPlaybook implements Playbook {
    @Override
    public String identifier() {
      return "plain";
    }

    @Override
    public ActivePlaybook run(OpenArena arena) {
      throw new UnsupportedOperationException("not used in this test");
    }
  }

  static final class ManagedRegisteredPlaybook implements ManagedPlaybook, PlaybookRegistration {
    @Override
    public String identifier() {
      return "managed-registered";
    }

    @Override
    public ActivePlaybook run(OpenArena arena) {
      throw new UnsupportedOperationException("not used in this test");
    }

    @Override
    public ObjectNode forRegisteredFfi() {
      return MAPPER.createObjectNode().put("identifier", identifier());
    }
  }

  static final class UnmanagedUnregisteredPlaybook implements UnmanagedPlaybook {
    @Override
    public String identifier() {
      return "unmanaged-unregistered";
    }

    @Override
    public ActivePlaybook run(OpenArena arena) {
      throw new UnsupportedOperationException("not used in this test");
    }
  }

  @Test
  void registerPlaybook_nullPlaybook_throwsIllegalArgumentException() {
    MatchBuilder builder = new MatchBuilder("match");
    assertThrows(IllegalArgumentException.class, () -> builder.registerPlaybook(null, true));
  }

  @Test
  void registerPlaybook_notManagedOrUnmanaged_throwsIllegalArgumentException() {
    MatchBuilder builder = new MatchBuilder("match");
    IllegalArgumentException error =
        assertThrows(
            IllegalArgumentException.class, () -> builder.registerPlaybook(new PlainPlaybook(), false));
    assertTrue(error.getMessage().contains("must implement ManagedPlaybook or UnmanagedPlaybook"));
  }

  @Test
  void registerPlaybook_unmanagedWithExecOnDependencyStartTrue_throwsIllegalArgumentException() {
    MatchBuilder builder = new MatchBuilder("match");
    IllegalArgumentException error =
        assertThrows(
            IllegalArgumentException.class,
            () -> builder.registerPlaybook(new UnmanagedUnregisteredPlaybook(), true));
    assertTrue(error.getMessage().contains("execOnDependencyStart=true"));
  }

  @Test
  void registerPlaybook_unmanagedWithExecOnDependencyStartFalse_registersSuccessfully() {
    MatchBuilder builder = new MatchBuilder("match");
    UnmanagedUnregisteredPlaybook playbook = new UnmanagedUnregisteredPlaybook();
    Match match = builder.registerPlaybook(playbook, false).build();
    assertEquals(playbook, match.playbook(UnmanagedUnregisteredPlaybook.class));
    assertFalse(match.execOnDependencyStart(UnmanagedUnregisteredPlaybook.class));
  }

  @Test
  void registerPlaybook_singleArgOverload_defaultsExecOnDependencyStartTrue() {
    MatchBuilder builder = new MatchBuilder("match");
    ManagedRegisteredPlaybook playbook = new ManagedRegisteredPlaybook();
    Match match = builder.registerPlaybook(playbook).build();
    assertTrue(match.execOnDependencyStart(ManagedRegisteredPlaybook.class));
  }

  @Test
  void registerPlaybook_duplicateClass_throwsIllegalStateException() {
    MatchBuilder builder = new MatchBuilder("match");
    builder.registerPlaybook(new ManagedRegisteredPlaybook());
    assertThrows(
        IllegalStateException.class, () -> builder.registerPlaybook(new ManagedRegisteredPlaybook()));
  }

  @Test
  void build_dependenciesAndComponentsAndNetwork_carriesThroughToMatch() {
    StubMatchPiece dependency = new StubMatchPiece();
    StubMatchPiece component = new StubMatchPiece();
    Match match =
        new MatchBuilder("match-name")
            .withNetwork("net")
            .addDependency(dependency)
            .addComponent(component)
            .build();
    assertEquals("match-name", match.name());
    ObjectNode ffi = match.forFfi();
    assertEquals("net", ffi.path("network").asText());
    assertEquals(1, ffi.path("dependencies").size());
    assertEquals(1, ffi.path("components").size());
  }
}
