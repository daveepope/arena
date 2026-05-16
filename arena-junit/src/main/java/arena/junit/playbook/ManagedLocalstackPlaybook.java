package arena.junit.playbook;

import arena.junit.OpenArena;
import arena.junit.support.ArenaJson;

import com.fasterxml.jackson.databind.node.ObjectNode;

public final class ManagedLocalstackPlaybook implements ArenaPlaybookRegistration, Playbook {
  private final String identifier;
  private final String dependencyIdentifier;

  public ManagedLocalstackPlaybook(String identifier, String dependencyIdentifier) {
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
    n.put("kind", "localstack");
    n.put("dependency_identifier", dependencyIdentifier);
    return n;
  }

  public ActiveLocalstackPlaybook run(OpenArena arena) {
    return new ActiveLocalstackPlaybook(dependencyIdentifier);
  }

  @Override
  public AutoCloseable enter(OpenArena arena) {
    ActiveLocalstackPlaybook p = run(arena);
    p.begin(arena);
    return p;
  }
}
