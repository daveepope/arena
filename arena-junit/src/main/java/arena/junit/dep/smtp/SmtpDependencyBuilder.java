package arena.junit.dep.smtp;
import arena.junit.match.ArenaMatchPiece;
import arena.junit.support.ArenaIdentifiers;
import arena.junit.support.ArenaJson;
import arena.junit.support.ChildrenFfi;

import com.fasterxml.jackson.databind.node.ObjectNode;
import java.util.ArrayList;
import java.util.List;

public final class SmtpDependencyBuilder {
  private final ObjectNode config =
      ArenaJson.object()
          .put("type", "smtp")
          .put("identifier", ArenaIdentifiers.build("arena-smtp", ""));
  private final List<ArenaMatchPiece> children = new ArrayList<>();

  public SmtpDependencyBuilder(String name) {
    config.put("identifier", ArenaIdentifiers.build("arena-smtp", name));
  }

  public SmtpDependencyBuilder withImageName(String imageName) {
    config.put("image_name", imageName);
    return this;
  }

  public SmtpDependencyBuilder withImage(String image) {
    config.put("image", image);
    return this;
  }

  public SmtpDependencyBuilder withPort(int port) {
    config.put("port", port);
    return this;
  }

  public SmtpDependencyBuilder withUiPort(int uiPort) {
    config.put("ui_port", uiPort);
    return this;
  }

  public SmtpDependencyBuilder withContainerName(String name) {
    config.put("container_name", name);
    return this;
  }

  public SmtpDependencyBuilder withStarttls() {
    config.put("tls_mode", "starttls");
    return this;
  }

  public SmtpDependencyBuilder withImplicitTls() {
    config.put("tls_mode", "implicit");
    return this;
  }

  public SmtpDependencyBuilder withChildDependencies(List<ArenaMatchPiece> children) {
    this.children.addAll(children);
    return this;
  }

  public SmtpDependency build() {
    ObjectNode cfg = config.deepCopy();
    if (!children.isEmpty()) {
      cfg.set("children", ChildrenFfi.build(children));
    }
    return new SmtpDependency(cfg);
  }
}
