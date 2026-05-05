package dev.arena.junit.playbook;
public final class ManagedMssqlPlaybookBuilder {
  private final String identifier;
  private final String dependencyIdentifier;

  public ManagedMssqlPlaybookBuilder(String identifier, String dependencyIdentifier) {
    this.identifier = identifier;
    this.dependencyIdentifier = dependencyIdentifier;
  }

  public ManagedMssqlPlaybook build() {
    return new ManagedMssqlPlaybook(identifier, dependencyIdentifier);
  }
}
