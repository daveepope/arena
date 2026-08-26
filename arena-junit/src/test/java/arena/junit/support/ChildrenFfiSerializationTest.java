package arena.junit.support;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;

import arena.junit.dep.HttpDependencyBuilder;
import arena.junit.dep.KafkaDependencyBuilder;
import arena.junit.dep.LocalstackDependencyBuilder;
import arena.junit.dep.MssqlDependencyBuilder;
import arena.junit.dep.PostgresDependencyBuilder;
import arena.junit.dep.smtp.SmtpDependencyBuilder;
import arena.junit.dep.temporal.TemporalDependencyBuilder;
import arena.junit.exec.ContainerizedComponentBuilder;
import arena.junit.exec.ExecutableComponentBuilder;
import arena.junit.match.ArenaRunnableDependency;
import arena.junit.oauth.OauthDependencyBuilder;

import com.fasterxml.jackson.databind.node.ObjectNode;

import java.util.function.Function;
import java.util.stream.Stream;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.Arguments;
import org.junit.jupiter.params.provider.MethodSource;

final class ChildrenFfiSerializationTest {

  @Test
  void httpDependency_noChildren_omitsChildrenKey() {
    ObjectNode config = new HttpDependencyBuilder("parent").build().forFfi();
    assertFalse(config.has("children"));
  }

  @Test
  void httpDependency_withChildDependency_nestsChildConfig() {
    var child = new HttpDependencyBuilder("child").withPort(9090).build();
    ObjectNode config =
        new HttpDependencyBuilder("parent")
            .addChildDependency(child)
            .build()
            .forFfi();
    assertEquals(1, config.path("children").size());
    assertEquals("http", config.path("children").get(0).path("type").asText());
    assertEquals(9090, config.path("children").get(0).path("port").asInt());
    assertEquals(child.identifier(), config.path("children").get(0).path("identifier").asText());
  }

  @Test
  void executableComponent_noChildren_omitsChildrenKey() {
    ObjectNode config =
        new ExecutableComponentBuilder("parent")
            .withExecutablePath("/bin/true")
            .build()
            .forFfi();
    assertFalse(config.has("children"));
  }

  @Test
  void executableComponent_withChildComponent_nestsChildConfig() {
    var child =
        new ExecutableComponentBuilder("child").withExecutablePath("/bin/true").build();
    ObjectNode config =
        new ExecutableComponentBuilder("parent")
            .withExecutablePath("/bin/true")
            .addChildComponent(child)
            .build()
            .forFfi();
    assertEquals(1, config.path("children").size());
    assertEquals("exec", config.path("children").get(0).path("type").asText());
  }

  @Test
  void containerizedComponent_noChildren_omitsChildrenKey() {
    ObjectNode config = new ContainerizedComponentBuilder("parent", "Dockerfile").build().forFfi();
    assertFalse(config.has("children"));
  }

  @Test
  void containerizedComponent_withChildComponent_nestsChildConfig() {
    var child = new ContainerizedComponentBuilder("child", "Dockerfile").build();
    ObjectNode config =
        new ContainerizedComponentBuilder("parent", "Dockerfile")
            .addChildComponent(child)
            .build()
            .forFfi();
    assertEquals(1, config.path("children").size());
    assertEquals("container", config.path("children").get(0).path("type").asText());
  }

  private static Stream<Arguments> remainingDependencyTypeFactories() {
    return Stream.of(
        Arguments.of("kafka", (Function<ArenaRunnableDependency, ArenaRunnableDependency>) child -> {
          var b = new KafkaDependencyBuilder("dep");
          if (child != null) {
            b.addChildDependency(child);
          }
          return b.build();
        }),
        Arguments.of("localstack", (Function<ArenaRunnableDependency, ArenaRunnableDependency>) child -> {
          var b = new LocalstackDependencyBuilder("dep");
          if (child != null) {
            b.addChildDependency(child);
          }
          return b.build();
        }),
        Arguments.of("mssql", (Function<ArenaRunnableDependency, ArenaRunnableDependency>) child -> {
          var b = new MssqlDependencyBuilder("dep");
          if (child != null) {
            b.addChildDependency(child);
          }
          return b.build();
        }),
        Arguments.of("oauth", (Function<ArenaRunnableDependency, ArenaRunnableDependency>) child -> {
          var b = new OauthDependencyBuilder("dep");
          if (child != null) {
            b.addChildDependency(child);
          }
          return b.build();
        }),
        Arguments.of("postgres", (Function<ArenaRunnableDependency, ArenaRunnableDependency>) child -> {
          var b = new PostgresDependencyBuilder("dep");
          if (child != null) {
            b.addChildDependency(child);
          }
          return b.build();
        }),
        Arguments.of("smtp", (Function<ArenaRunnableDependency, ArenaRunnableDependency>) child -> {
          var b = new SmtpDependencyBuilder("dep");
          if (child != null) {
            b.addChildDependency(child);
          }
          return b.build();
        }),
        Arguments.of("temporal", (Function<ArenaRunnableDependency, ArenaRunnableDependency>) child -> {
          var b = new TemporalDependencyBuilder("dep");
          if (child != null) {
            b.addChildDependency(child);
          }
          return b.build();
        }));
  }

  @ParameterizedTest
  @MethodSource("remainingDependencyTypeFactories")
  void dependency_noChildren_omitsChildrenKey(
      String expectedType, Function<ArenaRunnableDependency, ArenaRunnableDependency> factory) {
    ObjectNode config = factory.apply(null).forFfi();
    assertFalse(config.has("children"));
  }

  @ParameterizedTest
  @MethodSource("remainingDependencyTypeFactories")
  void dependency_withChild_nestsChildConfig(
      String expectedType, Function<ArenaRunnableDependency, ArenaRunnableDependency> factory) {
    ArenaRunnableDependency child = factory.apply(null);
    ObjectNode config = factory.apply(child).forFfi();
    assertEquals(1, config.path("children").size());
    assertEquals(expectedType, config.path("children").get(0).path("type").asText());
  }
}
