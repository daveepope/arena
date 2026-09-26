package arena.junit.dep;
import arena.junit.match.ArenaRunnableDependency;
import arena.junit.support.ArenaIdentifiers;
import arena.junit.support.ArenaJson;
import arena.junit.support.ChildrenFfi;

import com.fasterxml.jackson.databind.node.ArrayNode;
import com.fasterxml.jackson.databind.node.ObjectNode;
import java.util.ArrayList;
import java.util.List;

public final class KafkaDependencyBuilder {
  private final ObjectNode config =
      ArenaJson.object()
          .put("type", "kafka")
          .put("identifier", ArenaIdentifiers.build("arena-kafka", ""))
          .set("topics", ArenaJson.array());
  private final List<ArenaRunnableDependency> children = new ArrayList<>();

  public KafkaDependencyBuilder(String name) {
    config.put("identifier", ArenaIdentifiers.build("arena-kafka", name));
  }

  public KafkaDependencyBuilder withExpiry(java.time.Duration expiry) {
    config.put("expiry_seconds", expirySeconds(expiry));
    return this;
  }

  public KafkaDependencyBuilder withoutExpiry() {
    config.put("expiry_seconds", 0);
    return this;
  }

  public KafkaDependencyBuilder withImageName(String imageName) {
    config.put("image_name", imageName);
    return this;
  }

  public KafkaDependencyBuilder withTopic(String topic) {
    ((ArrayNode) config.get("topics")).add(topic);
    return this;
  }

  public KafkaDependencyBuilder withFlavor(KafkaFlavor flavor) {
    config.put("flavor", flavor.value());
    return this;
  }

  public KafkaDependencyBuilder withPort(int port) {
    config.put("port", port);
    return this;
  }

  public KafkaDependencyBuilder withContainerName(String name) {
    config.put("container_name", name);
    return this;
  }

  public KafkaDependencyBuilder addChildDependency(ArenaRunnableDependency child) {
    this.children.add(child);
    return this;
  }

  public KafkaDependency build() {
    ObjectNode cfg = config.deepCopy();
    if (!children.isEmpty()) {
      cfg.set("children", ChildrenFfi.buildDependencies(children));
    }
    return new KafkaDependency(cfg);
  }

  private static long expirySeconds(java.time.Duration expiry) {
    if (expiry.isNegative()) {
      throw new IllegalArgumentException("expiry must not be negative: " + expiry);
    }
    long seconds = expiry.toSeconds();
    return seconds == 0 && !expiry.isZero() ? 1 : seconds;
  }

}
