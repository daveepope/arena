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
import arena.junit.match.ArenaMatchPiece;
import arena.junit.oauth.OauthDependencyBuilder;

import com.fasterxml.jackson.databind.node.ObjectNode;

import java.util.List;
import java.util.function.BiFunction;
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
  void httpDependency_withChildDependencies_nestsChildConfig() {
    var child = new HttpDependencyBuilder("child").withPort(9090).build();
    ObjectNode config =
        new HttpDependencyBuilder("parent")
            .withChildDependencies(java.util.List.of(child))
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
  void executableComponent_withChildComponents_nestsChildConfig() {
    var child =
        new ExecutableComponentBuilder("child").withExecutablePath("/bin/true").build();
    ObjectNode config =
        new ExecutableComponentBuilder("parent")
            .withExecutablePath("/bin/true")
            .withChildComponents(java.util.List.of(child))
            .build()
            .forFfi();
    assertEquals(1, config.path("children").size());
    assertEquals("exec", config.path("children").get(0).path("type").asText());
  }

  private static Stream<Arguments> remainingTypeFactories() {
    return Stream.of(
        Arguments.of("kafka", (BiFunction<String, List<ArenaMatchPiece>, ArenaMatchPiece>)
            (name, children) -> new KafkaDependencyBuilder(name).withChildDependencies(children).build()),
        Arguments.of("localstack", (BiFunction<String, List<ArenaMatchPiece>, ArenaMatchPiece>)
            (name, children) -> new LocalstackDependencyBuilder(name).withChildDependencies(children).build()),
        Arguments.of("mssql", (BiFunction<String, List<ArenaMatchPiece>, ArenaMatchPiece>)
            (name, children) -> new MssqlDependencyBuilder(name).withChildDependencies(children).build()),
        Arguments.of("oauth", (BiFunction<String, List<ArenaMatchPiece>, ArenaMatchPiece>)
            (name, children) -> new OauthDependencyBuilder(name).withChildDependencies(children).build()),
        Arguments.of("postgres", (BiFunction<String, List<ArenaMatchPiece>, ArenaMatchPiece>)
            (name, children) -> new PostgresDependencyBuilder(name).withChildDependencies(children).build()),
        Arguments.of("smtp", (BiFunction<String, List<ArenaMatchPiece>, ArenaMatchPiece>)
            (name, children) -> new SmtpDependencyBuilder(name).withChildDependencies(children).build()),
        Arguments.of("temporal", (BiFunction<String, List<ArenaMatchPiece>, ArenaMatchPiece>)
            (name, children) -> new TemporalDependencyBuilder(name).withChildDependencies(children).build()),
        Arguments.of("container", (BiFunction<String, List<ArenaMatchPiece>, ArenaMatchPiece>)
            (name, children) -> new ContainerizedComponentBuilder(name, "Dockerfile").withChildComponents(children).build()));
  }

  @ParameterizedTest
  @MethodSource("remainingTypeFactories")
  void dependencyOrComponent_noChildren_omitsChildrenKey(
      String expectedType, BiFunction<String, List<ArenaMatchPiece>, ArenaMatchPiece> factory) {
    ObjectNode config = factory.apply("parent", List.of()).forFfi();
    assertFalse(config.has("children"));
  }

  @ParameterizedTest
  @MethodSource("remainingTypeFactories")
  void dependencyOrComponent_withChildren_nestsChildConfig(
      String expectedType, BiFunction<String, List<ArenaMatchPiece>, ArenaMatchPiece> factory) {
    ArenaMatchPiece child = factory.apply("child", List.of());
    ObjectNode config = factory.apply("parent", List.of(child)).forFfi();
    assertEquals(1, config.path("children").size());
    assertEquals(expectedType, config.path("children").get(0).path("type").asText());
  }
}
