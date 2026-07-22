package arena.junit.dep.temporal;
import arena.junit.support.ArenaIdentifiers;
import arena.junit.support.ArenaJson;

import com.fasterxml.jackson.databind.node.ObjectNode;

public final class TemporalDependencyBuilder {
  private final ObjectNode config =
      ArenaJson.object()
          .put("type", "temporal")
          .put("identifier", ArenaIdentifiers.build("arena-temporal", ""));

  public TemporalDependencyBuilder(String name) {
    config.put("identifier", ArenaIdentifiers.build("arena-temporal", name));
  }

  public TemporalDependencyBuilder withImageName(String imageName) {
    config.put("image_name", imageName);
    return this;
  }

  public TemporalDependencyBuilder withImage(String image) {
    config.put("image", image);
    return this;
  }

  public TemporalDependencyBuilder withPort(int port) {
    config.put("port", port);
    return this;
  }

  public TemporalDependencyBuilder withUiPort(int uiPort) {
    config.put("ui_port", uiPort);
    return this;
  }

  public TemporalDependencyBuilder withContainerName(String name) {
    config.put("container_name", name);
    return this;
  }

  public TemporalDependency build() {
    return new TemporalDependency(config.deepCopy());
  }
}
