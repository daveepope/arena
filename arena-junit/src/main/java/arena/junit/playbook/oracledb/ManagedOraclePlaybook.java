package arena.junit.playbook.oracledb;

import arena.junit.OpenArena;
import arena.junit.ffi.ArenaBindings;
import arena.junit.playbook.ManagedPlaybook;
import arena.junit.playbook.PlaybookRegistration;
import arena.junit.support.ArenaJson;

import com.fasterxml.jackson.databind.node.ObjectNode;
import com.sun.jna.Pointer;

public class ManagedOraclePlaybook implements ManagedPlaybook, PlaybookRegistration {
  private final String identifier;
  private final String dependencyIdentifier;

  protected ManagedOraclePlaybook(String identifier, String dependencyIdentifier) {
    if (identifier == null || identifier.isEmpty()) {
      throw new IllegalArgumentException("ManagedOraclePlaybook requires an identifier");
    }
    if (dependencyIdentifier == null || dependencyIdentifier.isEmpty()) {
      throw new IllegalArgumentException("ManagedOraclePlaybook requires a dependency identifier");
    }
    this.identifier = identifier;
    this.dependencyIdentifier = dependencyIdentifier;
  }

  @Override
  public String identifier() {
    return identifier;
  }

  public String dependencyIdentifier() {
    return dependencyIdentifier;
  }

  @Override
  public ObjectNode forRegisteredFfi() {
    ObjectNode n = ArenaJson.object();
    n.put("identifier", identifier);
    n.put("kind", "oracledb");
    n.put("dependency_identifier", dependencyIdentifier);
    return n;
  }

  @Override
  public ActiveOraclePlaybook run(OpenArena arena) {
    Pointer handle = ArenaBindings.matchPlaybookRun(arena.handle(), identifier);
    return new ActiveOraclePlaybook(handle);
  }
}
