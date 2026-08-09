package arena.junit.dep;
import arena.junit.match.ArenaMatchPiece;
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
  private final List<ArenaMatchPiece> children = new ArrayList<>();

  public KafkaDependencyBuilder(String name) {
    config.put("identifier", ArenaIdentifiers.build("arena-kafka", name));
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

  public KafkaDependencyBuilder withChildDependencies(List<ArenaMatchPiece> children) {
    this.children.addAll(children);
    return this;
  }

  public KafkaDependency build() {
    ObjectNode cfg = config.deepCopy();
    if (!children.isEmpty()) {
      cfg.set("children", ChildrenFfi.build(children));
    }
    return new KafkaDependency(cfg);
  }
}
