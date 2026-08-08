package arena.junit.match;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertSame;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import arena.junit.OpenArena;
import arena.junit.playbook.ActivePlaybook;
import arena.junit.playbook.ManagedPlaybook;
import arena.junit.playbook.PlaybookRegistration;
import arena.junit.playbook.UnmanagedPlaybook;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.node.ObjectNode;
import org.junit.jupiter.api.Test;

final class RegisteredPlaybookUnitTest {

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
  void constructor_execOnDependencyStartTrueWithRegistration_succeeds() {
    ManagedRegisteredPlaybook playbook = new ManagedRegisteredPlaybook();
    RegisteredPlaybook registered = new RegisteredPlaybook(playbook, true);
    assertSame(playbook, registered.playbook());
    assertTrue(registered.execOnDependencyStart());
  }

  @Test
  void constructor_execOnDependencyStartFalseWithoutRegistration_succeeds() {
    UnmanagedUnregisteredPlaybook playbook = new UnmanagedUnregisteredPlaybook();
    RegisteredPlaybook registered = new RegisteredPlaybook(playbook, false);
    assertSame(playbook, registered.playbook());
    assertFalse(registered.execOnDependencyStart());
  }

  @Test
  void constructor_execOnDependencyStartTrueWithoutRegistration_throwsIllegalArgumentException() {
    UnmanagedUnregisteredPlaybook playbook = new UnmanagedUnregisteredPlaybook();
    IllegalArgumentException error =
        assertThrows(IllegalArgumentException.class, () -> new RegisteredPlaybook(playbook, true));
    assertTrue(error.getMessage().contains("PlaybookRegistration"));
    assertTrue(error.getMessage().contains("execOnDependencyStart=true"));
  }

  @Test
  void forFfi_playbookWithoutRegistration_returnsNull() {
    RegisteredPlaybook registered = new RegisteredPlaybook(new UnmanagedUnregisteredPlaybook(), false);
    assertNull(registered.forFfi());
  }

  @Test
  void forFfi_playbookWithRegistration_includesExecOnDependencyStartFlag() {
    RegisteredPlaybook registered = new RegisteredPlaybook(new ManagedRegisteredPlaybook(), true);
    ObjectNode node = registered.forFfi();
    assertEquals("managed-registered", node.path("identifier").asText());
    assertTrue(node.path("exec_on_dependency_start").asBoolean());
  }
}
