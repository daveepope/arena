package arena.junit.playbook;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.fasterxml.jackson.databind.node.ObjectNode;

import org.junit.jupiter.api.Test;

final class ManagedMssqlPlaybookUnitTest {

  @Test
  void constructor_nullIdentifier_throwsIllegalArgumentException() {
    assertThrows(IllegalArgumentException.class, () -> new ManagedMssqlPlaybook(null, "dep-mssql"));
  }

  @Test
  void constructor_emptyIdentifier_throwsIllegalArgumentException() {
    assertThrows(IllegalArgumentException.class, () -> new ManagedMssqlPlaybook("", "dep-mssql"));
  }

  @Test
  void constructor_nullDependencyIdentifier_throwsIllegalArgumentException() {
    assertThrows(IllegalArgumentException.class, () -> new ManagedMssqlPlaybook("pb-mssql", null));
  }

  @Test
  void constructor_emptyDependencyIdentifier_throwsIllegalArgumentException() {
    assertThrows(IllegalArgumentException.class, () -> new ManagedMssqlPlaybook("pb-mssql", ""));
  }

  @Test
  void identifierAndDependencyIdentifier_validArgs_returnConstructedValues() {
    ManagedMssqlPlaybook playbook = new ManagedMssqlPlaybook("pb-mssql", "dep-mssql");
    assertEquals("pb-mssql", playbook.identifier());
    assertEquals("dep-mssql", playbook.dependencyIdentifier());
  }

  @Test
  void forRegisteredFfi_validArgs_serializesMssqlKindShape() {
    ManagedMssqlPlaybook playbook = new ManagedMssqlPlaybook("pb-mssql", "dep-mssql");
    ObjectNode n = playbook.forRegisteredFfi();
    assertEquals("pb-mssql", n.path("identifier").asText());
    assertEquals("mssql", n.path("kind").asText());
    assertEquals("dep-mssql", n.path("dependency_identifier").asText());
  }

  @Test
  void isManagedAndPlaybookRegistration_implementsExpectedInterfaces() {
    ManagedMssqlPlaybook playbook = new ManagedMssqlPlaybook("pb-mssql", "dep-mssql");
    assertTrue(playbook instanceof ManagedPlaybook);
    assertTrue(playbook instanceof PlaybookRegistration);
  }
}
