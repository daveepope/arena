package arena.junit.exec;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import arena.junit.match.ArenaRunnableComponent;
import arena.junit.readiness.HttpReadinessCheck;
import arena.junit.readiness.TcpReadinessCheck;
import arena.junit.support.ArenaJson;

import com.fasterxml.jackson.databind.node.ArrayNode;
import com.fasterxml.jackson.databind.node.ObjectNode;

import org.junit.jupiter.api.Test;

final class ContainerizedComponentBuilderSerializationTest {

  static final class StubComponent implements ArenaRunnableComponent {
    @Override
    public ObjectNode forFfi() {
      return ArenaJson.object().put("identifier", "child");
    }
  }

  @Test
  void build_minimalNameAndContainerfile_serializesTypeAndEmptyCollections() {
    ObjectNode config = new ContainerizedComponentBuilder("web", "Containerfile").build().forFfi();
    assertEquals("container", config.path("type").asText());
    assertTrue(config.path("identifier").asText().startsWith("arena-containerized-component-web-"));
    assertEquals("Containerfile", config.path("containerfile").asText());
    assertTrue(config.path("env_vars").isEmpty());
    assertTrue(config.path("runtime_args").isEmpty());
    assertTrue(config.path("port_mappings").isEmpty());
    assertTrue(config.path("host_mappings").isEmpty());
    assertFalse(config.has("readiness_checks"));
    assertFalse(config.has("children"));
  }

  @Test
  void withBuildContextNetworkAndImageTag_setsScalarFields() {
    ObjectNode config =
        new ContainerizedComponentBuilder("web", "Containerfile")
            .withBuildContext(".")
            .withNetwork("net")
            .withImageTag("v1")
            .build()
            .forFfi();
    assertEquals(".", config.path("build_context").asText());
    assertEquals("net", config.path("network").asText());
    assertEquals("v1", config.path("image_tag").asText());
  }

  @Test
  void withPortMapping_appendsHostAndContainerPorts() {
    ObjectNode config =
        new ContainerizedComponentBuilder("web", "Containerfile")
            .withPortMapping(8080, 80)
            .build()
            .forFfi();
    ObjectNode mapping = (ObjectNode) config.path("port_mappings").get(0);
    assertEquals(8080, mapping.path("host_port").asInt());
    assertEquals(80, mapping.path("container_port").asInt());
  }

  @Test
  void withHostMapping_appendsHostMappingsArray() {
    ObjectNode config =
        new ContainerizedComponentBuilder("web", "Containerfile")
            .withHostMapping("db.local:127.0.0.1")
            .build()
            .forFfi();
    assertEquals(
        "db.local:127.0.0.1", config.path("host_mappings").get(0).asText());
  }

  @Test
  void withEnvVar_setsEnvVarsObject() {
    ObjectNode config =
        new ContainerizedComponentBuilder("web", "Containerfile")
            .withEnvVar("LOG_LEVEL", "debug")
            .build()
            .forFfi();
    assertEquals("debug", config.path("env_vars").path("LOG_LEVEL").asText());
  }

  @Test
  void withRuntimeArg_appendsNameValuePair() {
    ObjectNode config =
        new ContainerizedComponentBuilder("web", "Containerfile")
            .withRuntimeArg("--cap-add", "NET_ADMIN")
            .build()
            .forFfi();
    ObjectNode pair = (ObjectNode) config.path("runtime_args").get(0);
    assertEquals("--cap-add", pair.path("name").asText());
    assertEquals("NET_ADMIN", pair.path("value").asText());
  }

  @Test
  void withReadinessCheck_defaultTimeout_usesTenSecondDefault() {
    ContainerizedComponent component =
        new ContainerizedComponentBuilder("web", "Containerfile")
            .withReadinessCheck(HttpReadinessCheck.create(), "/health")
            .build();
    assertEquals(10_000L, component.readinessEntries().getFirst().timeoutMs());
  }

  @Test
  void withReadinessCheck_httpAndTcp_serializeReadinessChecksArray() {
    ObjectNode config =
        new ContainerizedComponentBuilder("web", "Containerfile")
            .withReadinessCheck(HttpReadinessCheck.create(), "/health", 5_000L)
            .withReadinessCheck(TcpReadinessCheck.create(), "5432", 2_000L)
            .build()
            .forFfi();
    ArrayNode checks = (ArrayNode) config.path("readiness_checks");
    assertEquals("http", checks.get(0).path("kind").asText());
    assertEquals("/health", checks.get(0).path("target").asText());
    assertEquals(5_000L, checks.get(0).path("timeout_ms").asLong());
    assertEquals("tcp", checks.get(1).path("kind").asText());
    assertEquals("5432", checks.get(1).path("target").asText());
  }

  @Test
  void addChildComponent_nonEmptyChildren_serializesChildrenArray() {
    ObjectNode config =
        new ContainerizedComponentBuilder("web", "Containerfile")
            .addChildComponent(new StubComponent())
            .build()
            .forFfi();
    assertEquals(1, ((ArrayNode) config.path("children")).size());
  }

  @Test
  void identifier_returnsConfiguredIdentifier() {
    ContainerizedComponent component =
        new ContainerizedComponentBuilder("web", "Containerfile").build();
    assertTrue(component.identifier().startsWith("arena-containerized-component-web-"));
  }
}
