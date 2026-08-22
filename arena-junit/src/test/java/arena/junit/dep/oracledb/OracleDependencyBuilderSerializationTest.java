package arena.junit.dep.oracledb;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import arena.junit.match.ArenaRunnableDependency;
import arena.junit.support.ArenaJson;

import com.fasterxml.jackson.databind.node.ArrayNode;
import com.fasterxml.jackson.databind.node.ObjectNode;

import java.util.List;
import org.junit.jupiter.api.Test;

final class OracleDependencyBuilderSerializationTest {

  static final class StubDependency implements ArenaRunnableDependency {
    @Override
    public ObjectNode forFfi() {
      return ArenaJson.object().put("identifier", "child");
    }
  }

  @Test
  void build_minimalName_serializesTypeAndIdentifier() {
    ObjectNode config = new OracleDependencyBuilder("ora").build().forFfi();
    assertEquals("oracle", config.path("type").asText());
    assertTrue(config.path("identifier").asText().startsWith("arena-oracle-ora-"));
    assertFalse(config.has("children"));
  }

  @Test
  void build_minimalName_omitsAdminPasswordKey() {
    ObjectNode config = new OracleDependencyBuilder("ora").build().forFfi();
    assertFalse(config.has("admin_password"));
  }

  @Test
  void withOverrides_allFields_serializesConfiguredFields() {
    ObjectNode config =
        new OracleDependencyBuilder("ora")
            .withImage("21-slim")
            .withImageName("oracle-free")
            .withPort(1521)
            .withDatabaseName("arena")
            .withDatabaseUsername("arena_user")
            .withDatabasePassword("secret")
            .withAdminPassword("secret-admin")
            .withContainerName("ora-box")
            .build()
            .forFfi();
    assertEquals("21-slim", config.path("image").asText());
    assertEquals("oracle-free", config.path("image_name").asText());
    assertEquals(1521, config.path("port").asInt());
    assertEquals("arena", config.path("database_name").asText());
    assertEquals("arena_user", config.path("database_username").asText());
    assertEquals("secret", config.path("database_password").asText());
    assertEquals("secret-admin", config.path("admin_password").asText());
    assertEquals("ora-box", config.path("container_name").asText());
  }

  @Test
  void withStartupSqlScripts_multipleScripts_serializesInOrder() {
    ObjectNode config =
        new OracleDependencyBuilder("ora")
            .withStartupSqlScripts(List.of("seed.sql", "grants.sql"))
            .build()
            .forFfi();
    ArrayNode scripts = (ArrayNode) config.path("startup_sql_scripts");
    assertEquals(2, scripts.size());
    assertEquals("seed.sql", scripts.get(0).asText());
    assertEquals("grants.sql", scripts.get(1).asText());
  }

  @Test
  void addChildDependency_stubDependency_serializesChildrenViaChildrenFfi() {
    ObjectNode config =
        new OracleDependencyBuilder("ora")
            .addChildDependency(new StubDependency())
            .build()
            .forFfi();
    assertEquals(1, ((ArrayNode) config.path("children")).size());
  }

  @Test
  void identifier_afterBuild_returnsConfiguredIdentifier() {
    OracleDependency dep = new OracleDependencyBuilder("ora").build();
    assertTrue(dep.identifier().startsWith("arena-oracle-ora-"));
  }
}
