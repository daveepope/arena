package arena.junit.dep;
import arena.junit.support.ArenaIdentifiers;
import arena.junit.support.ArenaJson;

import com.fasterxml.jackson.databind.node.ArrayNode;
import com.fasterxml.jackson.databind.node.ObjectNode;
import java.util.List;

public final class MssqlDependencyBuilder {
  private final ObjectNode config =
      ArenaJson.object()
          .put("type", "mssql")
          .put("identifier", ArenaIdentifiers.build("arena-mssql", ""));

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

  public MssqlDependencyBuilder withStartupSqlScripts(List<String> scripts) {
    ArrayNode a = ArenaJson.array();
    for (String s : scripts) {
      a.add(s);
    }
    config.set("startup_sql_scripts", a);
    return this;
  }

  public MssqlDependency build() {
    return new MssqlDependency(config.deepCopy());
  }
}
