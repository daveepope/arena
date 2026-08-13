package arena.junit.playbook;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.fasterxml.jackson.databind.node.ObjectNode;

import org.junit.jupiter.api.Test;

final class ManagedPostgresPlaybookUnitTest {

  @Test
  void constructor_nullIdentifier_throwsIllegalArgumentException() {
    assertThrows(
        IllegalArgumentException.class, () -> new ManagedPostgresPlaybook(null, "dep-pg"));
  }

  @Test
  void constructor_emptyIdentifier_throwsIllegalArgumentException() {
    assertThrows(
        IllegalArgumentException.class, () -> new ManagedPostgresPlaybook("", "dep-pg"));
  }

  @Test
  void constructor_nullDependencyIdentifier_throwsIllegalArgumentException() {
    assertThrows(
        IllegalArgumentException.class, () -> new ManagedPostgresPlaybook("pb-pg", null));
  }

  @Test
  void constructor_emptyDependencyIdentifier_throwsIllegalArgumentException() {
    assertThrows(
        IllegalArgumentException.class, () -> new ManagedPostgresPlaybook("pb-pg", ""));
  }

  @Test
  void identifierAndDependencyIdentifier_validArgs_returnConstructedValues() {
    ManagedPostgresPlaybook playbook = new ManagedPostgresPlaybook("pb-pg", "dep-pg");
    assertEquals("pb-pg", playbook.identifier());
    assertEquals("dep-pg", playbook.dependencyIdentifier());
  }

  @Test
  void forRegisteredFfi_validArgs_serializesPostgresKindShape() {
    ManagedPostgresPlaybook playbook = new ManagedPostgresPlaybook("pb-pg", "dep-pg");
    ObjectNode n = playbook.forRegisteredFfi();
    assertEquals("pb-pg", n.path("identifier").asText());
    assertEquals("postgres", n.path("kind").asText());
    assertEquals("dep-pg", n.path("dependency_identifier").asText());
  }

  @Test
  void isManagedAndPlaybookRegistration_implementsExpectedInterfaces() {
    ManagedPostgresPlaybook playbook = new ManagedPostgresPlaybook("pb-pg", "dep-pg");
    assertTrue(playbook instanceof ManagedPlaybook);
    assertTrue(playbook instanceof PlaybookRegistration);
  }
}
