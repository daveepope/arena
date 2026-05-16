package arena.junit.playbook;

import arena.junit.OpenArena;
import arena.junit.support.ArenaJson;

import com.fasterxml.jackson.databind.node.ObjectNode;

public final class ManagedMssqlPlaybook implements ArenaPlaybookRegistration, Playbook {
  private final String identifier;
  private final String dependencyIdentifier;

  public ManagedMssqlPlaybook(String identifier, String dependencyIdentifier) {
    this.identifier = identifier;
    this.dependencyIdentifier = dependencyIdentifier;
  }

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

  public ActiveMssqlPlaybook run(OpenArena arena) {
    return new ActiveMssqlPlaybook(dependencyIdentifier);
  }

  @Override
  public AutoCloseable enter(OpenArena arena) {
    ActiveMssqlPlaybook p = run(arena);
    p.begin(arena);
    return p;
  }
}
