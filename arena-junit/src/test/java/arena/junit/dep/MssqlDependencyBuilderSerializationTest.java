package arena.junit.dep;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import arena.junit.match.ArenaRunnableDependency;
import arena.junit.support.ArenaJson;

import com.fasterxml.jackson.databind.node.ArrayNode;
import com.fasterxml.jackson.databind.node.ObjectNode;

import java.util.List;
import org.junit.jupiter.api.Test;

final class MssqlDependencyBuilderSerializationTest {

  static final class StubDependency implements ArenaRunnableDependency {
    @Override
    public ObjectNode forFfi() {
      return ArenaJson.object().put("identifier", "child");
    }
  }

  @Test
  void build_minimalName_serializesTypeAndIdentifier() {
    ObjectNode config = new MssqlDependencyBuilder("mssql").build().forFfi();
    assertEquals("mssql", config.path("type").asText());
    assertTrue(config.path("identifier").asText().startsWith("arena-mssql-mssql-"));
    assertFalse(config.has("children"));
  }

  @Test
  void withOverrides_allFields_serializesConfiguredFields() {
    ObjectNode config =
        new MssqlDependencyBuilder("mssql")
            .withImage("2022-latest")
            .withImageName("mcr.microsoft.com/mssql/server")
            .withPort(11433)
            .withDatabaseName("arena")
            .withDatabaseUsername("sa")
            .withDatabasePassword("secret")
            .withContainerName("mssql-box")
            .withEncryption(MssqlEncryption.ON)
            .build()
            .forFfi();
    assertEquals("2022-latest", config.path("image").asText());
    assertEquals("mcr.microsoft.com/mssql/server", config.path("image_name").asText());
    assertEquals(11433, config.path("port").asInt());
    assertEquals("arena", config.path("database_name").asText());
    assertEquals("sa", config.path("database_username").asText());
    assertEquals("secret", config.path("database_password").asText());
    assertEquals("mssql-box", config.path("container_name").asText());
    assertEquals("on", config.path("encryption").asText());
  }

  @Test
  void withEncryptionOff_serializesOffValue() {
    ObjectNode config =
        new MssqlDependencyBuilder("mssql").withEncryption(MssqlEncryption.OFF).build().forFfi();
    assertEquals("off", config.path("encryption").asText());
  }

  @Test
  void withStartupSqlScripts_appendsScriptsArray() {
    ObjectNode config =
        new MssqlDependencyBuilder("mssql")
            .withStartupSqlScripts(List.of("seed.sql", "grants.sql"))
            .build()
            .forFfi();
    ArrayNode scripts = (ArrayNode) config.path("startup_sql_scripts");
    assertEquals(2, scripts.size());
    assertEquals("seed.sql", scripts.get(0).asText());
    assertEquals("grants.sql", scripts.get(1).asText());
  }

  @Test
  void addChildDependency_nonEmptyChildren_serializesChildrenArray() {
    ObjectNode config =
        new MssqlDependencyBuilder("mssql").addChildDependency(new StubDependency()).build().forFfi();
    assertEquals(1, ((ArrayNode) config.path("children")).size());
  }

  @Test
  void identifier_returnsConfiguredIdentifier() {
    MssqlDependency dep = new MssqlDependencyBuilder("mssql").build();
    assertTrue(dep.identifier().startsWith("arena-mssql-mssql-"));
  }
}
