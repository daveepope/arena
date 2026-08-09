package arena.junit.dep;
import arena.junit.match.ArenaMatchPiece;
import arena.junit.support.ArenaIdentifiers;
import arena.junit.support.ArenaJson;
import arena.junit.support.ChildrenFfi;

import com.fasterxml.jackson.databind.node.ArrayNode;
import com.fasterxml.jackson.databind.node.ObjectNode;
import java.util.ArrayList;
import java.util.List;

public final class PostgresDependencyBuilder {
  private final ObjectNode config =
      ArenaJson.object()
          .put("type", "postgres")
          .put("identifier", ArenaIdentifiers.build("arena-postgres", ""));
  private final List<ArenaMatchPiece> children = new ArrayList<>();

  public PostgresDependencyBuilder(String name) {
    config.put("identifier", ArenaIdentifiers.build("arena-postgres", name));
  }

  public PostgresDependencyBuilder withImageName(String imageName) {
    config.put("image_name", imageName);
    return this;
  }

  public PostgresDependencyBuilder withImage(String image) {
    config.put("image", image);
    return this;
  }

  public PostgresDependencyBuilder withPort(int port) {
    config.put("port", port);
    return this;
  }

  public PostgresDependencyBuilder withDatabaseName(String name) {
    config.put("database_name", name);
    return this;
  }

  public PostgresDependencyBuilder withDatabaseUsername(String username) {
    config.put("database_username", username);
    return this;
  }

  public PostgresDependencyBuilder withDatabasePassword(String password) {
    config.put("database_password", password);
    return this;
  }

  public PostgresDependencyBuilder withContainerName(String name) {
    config.put("container_name", name);
    return this;
  }

  public PostgresDependencyBuilder withStartupSqlScripts(List<String> scripts) {
    ArrayNode a = ArenaJson.array();
    for (String s : scripts) {
      a.add(s);
    }
    config.set("startup_sql_scripts", a);
    return this;
  }

  public PostgresDependencyBuilder withChildDependencies(List<ArenaMatchPiece> children) {
    this.children.addAll(children);
    return this;
  }

  public PostgresDependency build() {
    ObjectNode cfg = config.deepCopy();
    if (!children.isEmpty()) {
      cfg.set("children", ChildrenFfi.build(children));
    }
    return new PostgresDependency(cfg);
  }
}
