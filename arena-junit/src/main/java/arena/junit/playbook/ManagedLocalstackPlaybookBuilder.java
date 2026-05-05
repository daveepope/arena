package arena.junit.playbook;
public final class ManagedLocalstackPlaybookBuilder {
  private final String identifier;
  private final String dependencyIdentifier;

  public ManagedLocalstackPlaybookBuilder(String identifier, String dependencyIdentifier) {
    this.identifier = identifier;
    this.dependencyIdentifier = dependencyIdentifier;
  }

  public ManagedLocalstackPlaybook build() {
    return new ManagedLocalstackPlaybook(identifier, dependencyIdentifier);
  }
}
