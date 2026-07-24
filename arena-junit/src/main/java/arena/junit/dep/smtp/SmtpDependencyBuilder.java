package arena.junit.dep.smtp;
import arena.junit.support.ArenaIdentifiers;
import arena.junit.support.ArenaJson;

import com.fasterxml.jackson.databind.node.ObjectNode;

public final class SmtpDependencyBuilder {
  private final ObjectNode config =
      ArenaJson.object()
          .put("type", "smtp")
          .put("identifier", ArenaIdentifiers.build("arena-smtp", ""));

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
    config.put("starttls", true);
    return this;
  }

  public SmtpDependency build() {
    return new SmtpDependency(config.deepCopy());
  }
}
