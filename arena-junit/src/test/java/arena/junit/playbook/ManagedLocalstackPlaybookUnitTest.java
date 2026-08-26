package arena.junit.playbook;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.fasterxml.jackson.databind.node.ObjectNode;

import org.junit.jupiter.api.Test;

final class ManagedLocalstackPlaybookUnitTest {

  @Test
  void constructor_nullIdentifier_throwsIllegalArgumentException() {
    assertThrows(
        IllegalArgumentException.class, () -> new ManagedLocalstackPlaybook(null, "dep-ls"));
  }

  @Test
  void constructor_emptyIdentifier_throwsIllegalArgumentException() {
    assertThrows(
        IllegalArgumentException.class, () -> new ManagedLocalstackPlaybook("", "dep-ls"));
  }

  @Test
  void constructor_nullDependencyIdentifier_throwsIllegalArgumentException() {
    assertThrows(
        IllegalArgumentException.class, () -> new ManagedLocalstackPlaybook("pb-ls", null));
  }

  @Test
  void constructor_emptyDependencyIdentifier_throwsIllegalArgumentException() {
    assertThrows(
        IllegalArgumentException.class, () -> new ManagedLocalstackPlaybook("pb-ls", ""));
  }

  @Test
  void identifierAndDependencyIdentifier_validArgs_returnConstructedValues() {
    ManagedLocalstackPlaybook playbook = new ManagedLocalstackPlaybook("pb-ls", "dep-ls");
    assertEquals("pb-ls", playbook.identifier());
    assertEquals("dep-ls", playbook.dependencyIdentifier());
  }

  @Test
  void forRegisteredFfi_validArgs_serializesLocalstackKindShape() {
    ManagedLocalstackPlaybook playbook = new ManagedLocalstackPlaybook("pb-ls", "dep-ls");
    ObjectNode n = playbook.forRegisteredFfi();
    assertEquals("pb-ls", n.path("identifier").asText());
    assertEquals("localstack", n.path("kind").asText());
    assertEquals("dep-ls", n.path("dependency_identifier").asText());
  }

  @Test
  void isManagedAndPlaybookRegistration_implementsExpectedInterfaces() {
    ManagedLocalstackPlaybook playbook = new ManagedLocalstackPlaybook("pb-ls", "dep-ls");
    assertTrue(playbook instanceof ManagedPlaybook);
    assertTrue(playbook instanceof PlaybookRegistration);
  }
}
