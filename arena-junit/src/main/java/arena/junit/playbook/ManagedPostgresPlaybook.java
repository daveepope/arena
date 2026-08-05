package arena.junit.playbook;

import arena.junit.OpenArena;
import arena.junit.ffi.ArenaBindings;
import arena.junit.support.ArenaJson;

import com.fasterxml.jackson.databind.node.ObjectNode;
import com.sun.jna.Pointer;

public class ManagedPostgresPlaybook implements Playbook, PlaybookRegistration {
  private final String identifier;
  private final String dependencyIdentifier;

  protected ManagedPostgresPlaybook(String identifier, String dependencyIdentifier) {
    if (identifier == null || identifier.isEmpty()) {
      throw new IllegalArgumentException("ManagedPostgresPlaybook requires an identifier");
    }
    if (dependencyIdentifier == null || dependencyIdentifier.isEmpty()) {
      throw new IllegalArgumentException("ManagedPostgresPlaybook requires a dependency identifier");
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
    n.put("kind", "postgres");
    n.put("dependency_identifier", dependencyIdentifier);
    return n;
  }

  @Override
  public ActivePostgresPlaybook run(OpenArena arena) {
    Pointer handle = ArenaBindings.matchPlaybookRun(arena.handle(), identifier);
    return new ActivePostgresPlaybook(handle);
  }
}
