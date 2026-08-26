package arena.junit.dep;
import arena.junit.match.ArenaRunnableDependency;
import arena.junit.playbook.HttpPlaybookBuilder;

import com.fasterxml.jackson.databind.node.ObjectNode;

public final class HttpDependency implements ArenaRunnableDependency {
  private final ObjectNode config;

  HttpDependency(ObjectNode config) {
    this.config = config;
  }

  public String identifier() {
    return config.get("identifier").asText();
  }

  public HttpPlaybookBuilder playbook() {
    return new HttpPlaybookBuilder(identifier());
  }

  @Override
  public ObjectNode forFfi() {
    return config;
  }
}
