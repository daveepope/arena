package arena.junit.dep.oracledb;
import arena.junit.match.ArenaRunnableDependency;
import arena.junit.support.ArenaIdentifiers;
import arena.junit.support.ArenaJson;
import arena.junit.support.ChildrenFfi;

import com.fasterxml.jackson.databind.node.ArrayNode;
import com.fasterxml.jackson.databind.node.ObjectNode;
import java.util.ArrayList;
import java.util.List;

public final class OracleDependencyBuilder {
  private final ObjectNode config =
      ArenaJson.object()
          .put("type", "oracle")
          .put("identifier", ArenaIdentifiers.build("arena-oracle", ""));
  private final List<ArenaRunnableDependency> children = new ArrayList<>();

  public OracleDependencyBuilder(String name) {
    config.put("identifier", ArenaIdentifiers.build("arena-oracle", name));
  }

  public OracleDependencyBuilder withImageName(String imageName) {
    config.put("image_name", imageName);
    return this;
  }

  public OracleDependencyBuilder withImage(String image) {
    config.put("image", image);
    return this;
  }

  public OracleDependencyBuilder withPort(int port) {
    config.put("port", port);
    return this;
  }

  public OracleDependencyBuilder withDatabaseName(String name) {
    config.put("database_name", name);
    return this;
  }

  public OracleDependencyBuilder withDatabaseUsername(String username) {
    config.put("database_username", username);
    return this;
  }

  public OracleDependencyBuilder withDatabasePassword(String password) {
    config.put("database_password", password);
    return this;
  }

  public OracleDependencyBuilder withAdminPassword(String password) {
    config.put("admin_password", password);
    return this;
  }

  public OracleDependencyBuilder withContainerName(String name) {
    config.put("container_name", name);
    return this;
  }

  public OracleDependencyBuilder withStartupSqlScripts(List<String> scripts) {
    ArrayNode a = ArenaJson.array();
    for (String s : scripts) {
      a.add(s);
    }
    config.set("startup_sql_scripts", a);
    return this;
  }

  public OracleDependencyBuilder addChildDependency(ArenaRunnableDependency child) {
    this.children.add(child);
    return this;
  }

  public OracleDependency build() {
    ObjectNode cfg = config.deepCopy();
    if (!children.isEmpty()) {
      cfg.set("children", ChildrenFfi.buildDependencies(children));
    }
    return new OracleDependency(cfg);
  }
}
