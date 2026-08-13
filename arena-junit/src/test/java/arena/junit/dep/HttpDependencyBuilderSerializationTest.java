package arena.junit.dep;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import arena.junit.match.ArenaRunnableDependency;
import arena.junit.playbook.HttpPlaybookBuilder;
import arena.junit.playbook.HttpResponse;
import arena.junit.support.ArenaJson;

import com.fasterxml.jackson.databind.node.ArrayNode;
import com.fasterxml.jackson.databind.node.ObjectNode;

import org.junit.jupiter.api.Test;

final class HttpDependencyBuilderSerializationTest {

  static final class StubDependency implements ArenaRunnableDependency {
    @Override
    public ObjectNode forFfi() {
      return ArenaJson.object().put("identifier", "child");
    }
  }

  @Test
  void build_minimalName_serializesTypeAndIdentifier() {
    ObjectNode config = new HttpDependencyBuilder("http").build().forFfi();
    assertEquals("http", config.path("type").asText());
    assertTrue(config.path("identifier").asText().startsWith("arena-http-http-"));
    assertFalse(config.has("children"));
  }

  @Test
  void withOverrides_allFields_serializesConfiguredFields() {
    ObjectNode config =
        new HttpDependencyBuilder("http")
            .withPort(18080)
            .withContainerName("http-box")
            .withImageName("example/http-stub")
            .withImageTag("3.9.1")
            .build()
            .forFfi();
    assertEquals(18080, config.path("port").asInt());
    assertEquals("http-box", config.path("container_name").asText());
    assertEquals("example/http-stub", config.path("image_name").asText());
    assertEquals("3.9.1", config.path("image_tag").asText());
  }

  @Test
  void addChildDependency_nonEmptyChildren_serializesChildrenArray() {
    ObjectNode config =
        new HttpDependencyBuilder("http").addChildDependency(new StubDependency()).build().forFfi();
    assertEquals(1, ((ArrayNode) config.path("children")).size());
  }

  @Test
  void identifier_returnsConfiguredIdentifier() {
    HttpDependency dep = new HttpDependencyBuilder("http").build();
    assertTrue(dep.identifier().startsWith("arena-http-http-"));
  }

  @Test
  void playbook_returnsBuilderSeededWithDependencyIdentifier() {
    HttpDependency dep = new HttpDependencyBuilder("http").build();
    HttpPlaybookBuilder builder = dep.playbook();
    builder.get("/health").willReturn(HttpResponse.ok()).intoPlaybook();
    assertEquals("GET", builder.mappingsForFfi().getFirst().path("method").asText());
  }
}
