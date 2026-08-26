package arena.junit.dep.smtp;
import arena.junit.match.ArenaRunnableDependency;

import com.fasterxml.jackson.databind.node.ObjectNode;

public final class SmtpDependency implements ArenaRunnableDependency {
  private final ObjectNode config;

  SmtpDependency(ObjectNode config) {
    this.config = config;
  }

  public String identifier() {
    return config.get("identifier").asText();
  }

  @Override
  public ObjectNode forFfi() {
    return config;
  }
}
