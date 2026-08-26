package arena.junit.exec;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import arena.junit.match.ArenaRunnableComponent;
import arena.junit.readiness.TcpReadinessCheck;
import arena.junit.support.ArenaJson;

import com.fasterxml.jackson.databind.node.ArrayNode;
import com.fasterxml.jackson.databind.node.ObjectNode;

import java.util.List;
import org.junit.jupiter.api.Test;

final class ExecutableComponentBuilderSerializationTest {

  static final class StubComponent implements ArenaRunnableComponent {
    @Override
    public ObjectNode forFfi() {
      return ArenaJson.object().put("identifier", "child");
    }
  }

  @Test
  void build_minimalName_serializesTypeAndEmptyCollections() {
    ObjectNode config = new ExecutableComponentBuilder("worker").build().forFfi();
    assertEquals("exec", config.path("type").asText());
    assertTrue(config.path("identifier").asText().startsWith("arena-executable-component-worker-"));
    assertTrue(config.path("env_vars").isEmpty());
    assertTrue(config.path("runtime_args").isEmpty());
    assertFalse(config.has("children"));
  }

  @Test
  void withExecutableAndSourcePath_setsScalarFields() {
    ObjectNode config =
        new ExecutableComponentBuilder("worker")
            .withExecutablePath("/bin/worker")
            .withSourcePath("./worker")
            .build()
            .forFfi();
    assertEquals("/bin/worker", config.path("executable_path").asText());
    assertEquals("./worker", config.path("source_path").asText());
  }

  @Test
  void withBuildTool_cargo_serializesToolValue() {
    ObjectNode config =
        new ExecutableComponentBuilder("worker").withBuildTool(BuildTool.CARGO).build().forFfi();
    assertEquals("cargo", config.path("build_tool").asText());
  }

  @Test
  void withBuildToolCustom_serializesCommandAndArgs() {
    ObjectNode config =
        new ExecutableComponentBuilder("worker")
            .withBuildToolCustom("./build.sh", List.of("--release", "--target=worker"))
            .build()
            .forFfi();
    ObjectNode buildTool = (ObjectNode) config.path("build_tool");
    assertEquals("./build.sh", buildTool.path("command").asText());
    ArrayNode args = (ArrayNode) buildTool.path("args");
    assertEquals("--release", args.get(0).asText());
    assertEquals("--target=worker", args.get(1).asText());
  }

  @Test
  void withEnvVar_setsEnvVarsObject() {
    ObjectNode config =
        new ExecutableComponentBuilder("worker").withEnvVar("RUST_LOG", "debug").build().forFfi();
    assertEquals("debug", config.path("env_vars").path("RUST_LOG").asText());
  }

  @Test
  void withRuntimeArg_appendsNameValuePair() {
    ObjectNode config =
        new ExecutableComponentBuilder("worker")
            .withRuntimeArg("--port", "8080")
            .build()
            .forFfi();
    ObjectNode pair = (ObjectNode) config.path("runtime_args").get(0);
    assertEquals("--port", pair.path("name").asText());
    assertEquals("8080", pair.path("value").asText());
  }

  @Test
  void withReadinessCheck_defaultTimeout_usesTenSecondDefault() {
    ExecutableComponent component =
        new ExecutableComponentBuilder("worker")
            .withReadinessCheck(TcpReadinessCheck.create(), "8080")
            .build();
    assertEquals(10_000L, component.readinessEntries().getFirst().timeoutMs());
  }

  @Test
  void withReadinessCheck_explicitTimeout_serializesReadinessChecksArray() {
    ObjectNode config =
        new ExecutableComponentBuilder("worker")
            .withReadinessCheck(TcpReadinessCheck.create(), "8080", 3_000L)
            .build()
            .forFfi();
    ObjectNode check = (ObjectNode) config.path("readiness_checks").get(0);
    assertEquals("tcp", check.path("kind").asText());
    assertEquals("8080", check.path("target").asText());
    assertEquals(3_000L, check.path("timeout_ms").asLong());
  }

  @Test
  void addChildComponent_nonEmptyChildren_serializesChildrenArray() {
    ObjectNode config =
        new ExecutableComponentBuilder("worker").addChildComponent(new StubComponent()).build().forFfi();
    assertEquals(1, ((ArrayNode) config.path("children")).size());
  }

  @Test
  void identifier_returnsConfiguredIdentifier() {
    ExecutableComponent component = new ExecutableComponentBuilder("worker").build();
    assertTrue(component.identifier().startsWith("arena-executable-component-worker-"));
  }

  @Test
  void customBuild_commandAndArgs_serializesCommandNode() {
    ObjectNode node = BuildTool.customBuild("./build.sh", List.of("--flag"));
    assertEquals("./build.sh", node.path("command").asText());
    assertEquals("--flag", node.path("args").get(0).asText());
  }
}
