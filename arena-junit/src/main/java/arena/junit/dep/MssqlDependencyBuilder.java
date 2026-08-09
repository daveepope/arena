package arena.junit.dep;
import arena.junit.match.ArenaMatchPiece;
import arena.junit.support.ArenaIdentifiers;
import arena.junit.support.ArenaJson;
import arena.junit.support.ChildrenFfi;

import com.fasterxml.jackson.databind.node.ArrayNode;
import com.fasterxml.jackson.databind.node.ObjectNode;
import java.util.ArrayList;
import java.util.List;

public final class MssqlDependencyBuilder {
  private final ObjectNode config =
      ArenaJson.object()
          .put("type", "mssql")
          .put("identifier", ArenaIdentifiers.build("arena-mssql", ""));
  private final List<ArenaMatchPiece> children = new ArrayList<>();

  public MssqlDependencyBuilder(String name) {
    config.put("identifier", ArenaIdentifiers.build("arena-mssql", name));
  }

  public MssqlDependencyBuilder withImageName(String imageName) {
    config.put("image_name", imageName);
    return this;
  }

  public MssqlDependencyBuilder withImage(String image) {
    config.put("image", image);
    return this;
  }

  public MssqlDependencyBuilder withPort(int port) {
    config.put("port", port);
    return this;
  }

  public MssqlDependencyBuilder withDatabaseName(String name) {
    config.put("database_name", name);
    return this;
  }

  public MssqlDependencyBuilder withDatabaseUsername(String username) {
    config.put("database_username", username);
    return this;
  }

  public MssqlDependencyBuilder withDatabasePassword(String password) {
    config.put("database_password", password);
    return this;
  }

  public MssqlDependencyBuilder withContainerName(String name) {
    config.put("container_name", name);
    return this;
  }

  public MssqlDependencyBuilder withEncryption(MssqlEncryption encryption) {
    config.put("encryption", encryption.value());
    return this;
  }

  public MssqlDependencyBuilder withStartupSqlScripts(List<String> scripts) {
    ArrayNode a = ArenaJson.array();
    for (String s : scripts) {
      a.add(s);
    }
    config.set("startup_sql_scripts", a);
    return this;
  }

  public MssqlDependencyBuilder withChildDependencies(List<ArenaMatchPiece> children) {
    this.children.addAll(children);
    return this;
  }

  public MssqlDependency build() {
    ObjectNode cfg = config.deepCopy();
    if (!children.isEmpty()) {
      cfg.set("children", ChildrenFfi.build(children));
    }
    return new MssqlDependency(cfg);
  }
}
