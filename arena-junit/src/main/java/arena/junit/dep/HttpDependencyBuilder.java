package arena.junit.dep;
import arena.junit.match.ArenaRunnableDependency;
import arena.junit.support.ArenaIdentifiers;
import arena.junit.support.ArenaJson;
import arena.junit.support.ChildrenFfi;

import com.fasterxml.jackson.databind.node.ObjectNode;
import java.util.ArrayList;
import java.util.List;

public final class HttpDependencyBuilder {
  private final ObjectNode config =
      ArenaJson.object()
          .put("type", "http")
          .put("identifier", ArenaIdentifiers.build("arena-http", ""));
  private final List<ArenaRunnableDependency> children = new ArrayList<>();

  public HttpDependencyBuilder(String name) {
    config.put("identifier", ArenaIdentifiers.build("arena-http", name));
  }

  public HttpDependencyBuilder withExpiry(java.time.Duration expiry) {
    config.put("expiry_seconds", expirySeconds(expiry));
    return this;
  }

  public HttpDependencyBuilder withoutExpiry() {
    config.put("expiry_seconds", 0);
    return this;
  }

  public HttpDependencyBuilder withPort(int port) {
    config.put("port", port);
    return this;
  }

  public HttpDependencyBuilder withContainerName(String name) {
    config.put("container_name", name);
    return this;
  }

  public HttpDependencyBuilder withImageName(String imageName) {
    config.put("image_name", imageName);
    return this;
  }

  public HttpDependencyBuilder withImageTag(String imageTag) {
    config.put("image_tag", imageTag);
    return this;
  }

  public HttpDependencyBuilder addChildDependency(ArenaRunnableDependency child) {
    this.children.add(child);
    return this;
  }

  public HttpDependency build() {
    ObjectNode cfg = config.deepCopy();
    if (!children.isEmpty()) {
      cfg.set("children", ChildrenFfi.buildDependencies(children));
    }
    return new HttpDependency(cfg);
  }

  private static long expirySeconds(java.time.Duration expiry) {
    if (expiry.isNegative()) {
      throw new IllegalArgumentException("expiry must not be negative: " + expiry);
    }
    long seconds = expiry.toSeconds();
    return seconds == 0 && !expiry.isZero() ? 1 : seconds;
  }

}
