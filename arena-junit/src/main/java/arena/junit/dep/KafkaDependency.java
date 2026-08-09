package arena.junit.dep;
import arena.junit.match.ArenaRunnableDependency;

import com.fasterxml.jackson.databind.node.ObjectNode;

public final class KafkaDependency implements ArenaRunnableDependency {
  public static final int KAFKA_INTERNAL_DOCKER_PORT = 29092;

  private final ObjectNode config;

  KafkaDependency(ObjectNode config) {
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
