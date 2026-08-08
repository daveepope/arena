package arena.junit.match;

import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

import arena.junit.OpenArena;
import arena.junit.playbook.ActivePlaybook;
import arena.junit.playbook.ManagedPlaybook;
import arena.junit.playbook.PlaybookRegistration;
import arena.junit.playbook.UnmanagedPlaybook;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.node.ObjectNode;
import org.junit.jupiter.api.Test;

final class MatchUnitTest {

  private static final ObjectMapper MAPPER = new ObjectMapper();

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
  void forFfi_noPlaybooksRegistered_omitsPlaybooksField() {
    Match match = new MatchBuilder("match").build();
    assertFalse(match.forFfi().has("playbooks"));
  }

  @Test
  void forFfi_managedPlaybookRegistered_includesPlaybooksField() {
    Match match = new MatchBuilder("match").registerPlaybook(new ManagedRegisteredPlaybook()).build();
    ObjectNode ffi = match.forFfi();
    assertTrue(ffi.has("playbooks"));
    assertTrue(ffi.path("playbooks").get(0).path("exec_on_dependency_start").asBoolean());
  }

  @Test
  void forFfi_unmanagedPlaybookWithoutRegistration_omitsPlaybooksFieldDespiteRegistration() {
    Match match =
        new MatchBuilder("match").registerPlaybook(new UnmanagedUnregisteredPlaybook(), false).build();
    assertFalse(
        match.forFfi().has("playbooks"),
        "playbooks not implementing PlaybookRegistration serialize to null and must be filtered out");
  }

  @Test
  void playbook_unregisteredClass_returnsNull() {
    Match match = new MatchBuilder("match").build();
    assertNull(match.playbook(ManagedRegisteredPlaybook.class));
  }

  @Test
  void execOnDependencyStart_unregisteredClass_returnsNull() {
    Match match = new MatchBuilder("match").build();
    assertNull(match.execOnDependencyStart(ManagedRegisteredPlaybook.class));
  }
}
