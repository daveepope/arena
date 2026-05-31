package arena.junit.playbook;

import arena.junit.OpenArena;
import arena.junit.ffi.ArenaBindings;
import arena.junit.support.ArenaJson;

import com.fasterxml.jackson.databind.node.ObjectNode;
import com.sun.jna.Pointer;

public class ManagedMssqlPlaybook implements Playbook, PlaybookRegistration {
  private final String identifier;
  private final String dependencyIdentifier;

  protected ManagedMssqlPlaybook(String identifier, String dependencyIdentifier) {
    if (identifier == null || identifier.isEmpty()) {
      throw new IllegalArgumentException("ManagedMssqlPlaybook requires an identifier");
    }
    if (dependencyIdentifier == null || dependencyIdentifier.isEmpty()) {
      throw new IllegalArgumentException("ManagedMssqlPlaybook requires a dependency identifier");
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
    n.put("kind", "mssql");
    n.put("dependency_identifier", dependencyIdentifier);
    return n;
  }

  @Override
  public ActiveMssqlPlaybook run(OpenArena arena) {
    Pointer handle = ArenaBindings.matchPlaybookRun(arena.handle(), identifier);
    return new ActiveMssqlPlaybook(handle);
  }
}
