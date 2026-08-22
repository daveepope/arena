package arena.junit.playbook.oracledb;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import arena.junit.playbook.ManagedPlaybook;
import arena.junit.playbook.PlaybookRegistration;

import com.fasterxml.jackson.databind.node.ObjectNode;

import org.junit.jupiter.api.Test;

final class ManagedOraclePlaybookUnitTest {

  @Test
  void constructor_nullIdentifier_throwsIllegalArgumentException() {
    assertThrows(
        IllegalArgumentException.class, () -> new ManagedOraclePlaybook(null, "dep-ora"));
  }

  @Test
  void constructor_emptyIdentifier_throwsIllegalArgumentException() {
    assertThrows(
        IllegalArgumentException.class, () -> new ManagedOraclePlaybook("", "dep-ora"));
  }

  @Test
  void constructor_nullDependencyIdentifier_throwsIllegalArgumentException() {
    assertThrows(
        IllegalArgumentException.class, () -> new ManagedOraclePlaybook("pb-ora", null));
  }

  @Test
  void constructor_emptyDependencyIdentifier_throwsIllegalArgumentException() {
    assertThrows(
        IllegalArgumentException.class, () -> new ManagedOraclePlaybook("pb-ora", ""));
  }

  @Test
  void identifierAndDependencyIdentifier_validArgs_returnConstructedValues() {
    ManagedOraclePlaybook playbook = new ManagedOraclePlaybook("pb-ora", "dep-ora");
    assertEquals("pb-ora", playbook.identifier());
    assertEquals("dep-ora", playbook.dependencyIdentifier());
  }

  @Test
  void forRegisteredFfi_validArgs_serializesOracleKindShape() {
    ManagedOraclePlaybook playbook = new ManagedOraclePlaybook("pb-ora", "dep-ora");
    ObjectNode n = playbook.forRegisteredFfi();
    assertEquals("pb-ora", n.path("identifier").asText());
    assertEquals("oracle", n.path("kind").asText());
    assertEquals("dep-ora", n.path("dependency_identifier").asText());
  }

  @Test
  void isManagedAndPlaybookRegistration_implementsExpectedInterfaces() {
    ManagedOraclePlaybook playbook = new ManagedOraclePlaybook("pb-ora", "dep-ora");
    assertTrue(playbook instanceof ManagedPlaybook);
    assertTrue(playbook instanceof PlaybookRegistration);
  }
}
