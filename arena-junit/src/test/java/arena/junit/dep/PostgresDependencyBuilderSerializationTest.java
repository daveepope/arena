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

final class PostgresDependencyBuilderSerializationTest {

  static final class StubDependency implements ArenaRunnableDependency {
    @Override
    public ObjectNode forFfi() {
      return ArenaJson.object().put("identifier", "child");
    }
  }

  @Test
  void build_minimalName_serializesTypeAndIdentifier() {
    ObjectNode config = new PostgresDependencyBuilder("pg").build().forFfi();
    assertEquals("postgres", config.path("type").asText());
    assertTrue(config.path("identifier").asText().startsWith("arena-postgres-pg-"));
    assertFalse(config.has("children"));
  }

  @Test
  void withOverrides_allFields_serializesConfiguredFields() {
    ObjectNode config =
        new PostgresDependencyBuilder("pg")
            .withImage("16-alpine")
            .withImageName("postgres")
            .withPort(15432)
            .withDatabaseName("arena")
            .withDatabaseUsername("arena_user")
            .withDatabasePassword("secret")
            .withContainerName("pg-box")
            .build()
            .forFfi();
    assertEquals("16-alpine", config.path("image").asText());
    assertEquals("postgres", config.path("image_name").asText());
    assertEquals(15432, config.path("port").asInt());
    assertEquals("arena", config.path("database_name").asText());
    assertEquals("arena_user", config.path("database_username").asText());
    assertEquals("secret", config.path("database_password").asText());
    assertEquals("pg-box", config.path("container_name").asText());
  }

  @Test
  void withStartupSqlScripts_appendsScriptsArray() {
    ObjectNode config =
        new PostgresDependencyBuilder("pg")
            .withStartupSqlScripts(List.of("seed.sql"))
            .build()
            .forFfi();
    ArrayNode scripts = (ArrayNode) config.path("startup_sql_scripts");
    assertEquals(1, scripts.size());
    assertEquals("seed.sql", scripts.get(0).asText());
  }

  @Test
  void addChildDependency_nonEmptyChildren_serializesChildrenArray() {
    ObjectNode config =
        new PostgresDependencyBuilder("pg")
            .addChildDependency(new StubDependency())
            .build()
            .forFfi();
    assertEquals(1, ((ArrayNode) config.path("children")).size());
  }

  @Test
  void identifier_returnsConfiguredIdentifier() {
    PostgresDependency dep = new PostgresDependencyBuilder("pg").build();
    assertTrue(dep.identifier().startsWith("arena-postgres-pg-"));
  }
}
