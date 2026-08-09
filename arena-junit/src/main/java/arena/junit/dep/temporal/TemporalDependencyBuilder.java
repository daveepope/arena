package arena.junit.dep.temporal;
import arena.junit.match.ArenaRunnableDependency;
import arena.junit.support.ArenaIdentifiers;
import arena.junit.support.ArenaJson;
import arena.junit.support.ChildrenFfi;

import com.fasterxml.jackson.databind.node.ObjectNode;
import java.util.ArrayList;
import java.util.List;

public final class TemporalDependencyBuilder {
  private final ObjectNode config =
      ArenaJson.object()
          .put("type", "temporal")
          .put("identifier", ArenaIdentifiers.build("arena-temporal", ""));
  private final List<ArenaRunnableDependency> children = new ArrayList<>();

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

  public TemporalDependencyBuilder addChildDependency(ArenaRunnableDependency child) {
    this.children.add(child);
    return this;
  }

  public TemporalDependency build() {
    ObjectNode cfg = config.deepCopy();
    if (!children.isEmpty()) {
      cfg.set("children", ChildrenFfi.buildDependencies(children));
    }
    return new TemporalDependency(cfg);
  }
}
