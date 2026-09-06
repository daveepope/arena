package arena.junit.dep;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertThrows;

import arena.junit.dep.oracledb.OracleDependencyBuilder;
import arena.junit.dep.smtp.SmtpDependencyBuilder;
import arena.junit.dep.temporal.TemporalDependencyBuilder;
import arena.junit.exec.ContainerizedComponentBuilder;
import com.fasterxml.jackson.databind.node.ObjectNode;
import java.time.Duration;
import java.util.function.Function;
import java.util.function.Supplier;
import java.util.stream.Stream;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.MethodSource;

final class ExpiryOverrideUnitTest {

  private record BuilderCase(
      String name,
      Supplier<ObjectNode> defaults,
      Function<Duration, ObjectNode> withExpiry,
      Supplier<ObjectNode> withoutExpiry) {

    @Override
    public String toString() {
      return name;
    }
  }

  static Stream<BuilderCase> builderCases() {
    return Stream.of(
        new BuilderCase(
            "http",
            () -> new HttpDependencyBuilder("orders").build().forFfi(),
            expiry -> new HttpDependencyBuilder("orders").withExpiry(expiry).build().forFfi(),
            () -> new HttpDependencyBuilder("orders").withoutExpiry().build().forFfi()),
        new BuilderCase(
            "kafka",
            () -> new KafkaDependencyBuilder("orders").build().forFfi(),
            expiry -> new KafkaDependencyBuilder("orders").withExpiry(expiry).build().forFfi(),
            () -> new KafkaDependencyBuilder("orders").withoutExpiry().build().forFfi()),
        new BuilderCase(
            "localstack",
            () -> new LocalstackDependencyBuilder("orders").build().forFfi(),
            expiry -> new LocalstackDependencyBuilder("orders").withExpiry(expiry).build().forFfi(),
            () -> new LocalstackDependencyBuilder("orders").withoutExpiry().build().forFfi()),
        new BuilderCase(
            "mssql",
            () -> new MssqlDependencyBuilder("orders").build().forFfi(),
            expiry -> new MssqlDependencyBuilder("orders").withExpiry(expiry).build().forFfi(),
            () -> new MssqlDependencyBuilder("orders").withoutExpiry().build().forFfi()),
        new BuilderCase(
            "postgres",
            () -> new PostgresDependencyBuilder("orders").build().forFfi(),
            expiry -> new PostgresDependencyBuilder("orders").withExpiry(expiry).build().forFfi(),
            () -> new PostgresDependencyBuilder("orders").withoutExpiry().build().forFfi()),
        new BuilderCase(
            "oracle",
            () -> new OracleDependencyBuilder("orders").build().forFfi(),
            expiry -> new OracleDependencyBuilder("orders").withExpiry(expiry).build().forFfi(),
            () -> new OracleDependencyBuilder("orders").withoutExpiry().build().forFfi()),
        new BuilderCase(
            "smtp",
            () -> new SmtpDependencyBuilder("orders").build().forFfi(),
            expiry -> new SmtpDependencyBuilder("orders").withExpiry(expiry).build().forFfi(),
            () -> new SmtpDependencyBuilder("orders").withoutExpiry().build().forFfi()),
        new BuilderCase(
            "temporal",
            () -> new TemporalDependencyBuilder("orders").build().forFfi(),
            expiry -> new TemporalDependencyBuilder("orders").withExpiry(expiry).build().forFfi(),
            () -> new TemporalDependencyBuilder("orders").withoutExpiry().build().forFfi()),
        new BuilderCase(
            "containerized-component",
            () -> new ContainerizedComponentBuilder("web", "Containerfile").build().forFfi(),
            expiry ->
                new ContainerizedComponentBuilder("web", "Containerfile")
                    .withExpiry(expiry)
                    .build()
                    .forFfi(),
            () ->
                new ContainerizedComponentBuilder("web", "Containerfile")
                    .withoutExpiry()
                    .build()
                    .forFfi()));
  }

  @ParameterizedTest
  @MethodSource("builderCases")
  void build_withoutExpiryOverride_omitsExpirySeconds(BuilderCase builder) {
    assertFalse(builder.defaults().get().has("expiry_seconds"));
  }

  @ParameterizedTest
  @MethodSource("builderCases")
  void withExpiry_thirtySeconds_setsExpirySeconds(BuilderCase builder) {
    ObjectNode config = builder.withExpiry().apply(Duration.ofSeconds(30));

    assertEquals(30, config.path("expiry_seconds").asLong());
  }

  @ParameterizedTest
  @MethodSource("builderCases")
  void withoutExpiry_called_setsExpirySecondsToZero(BuilderCase builder) {
    assertEquals(0, builder.withoutExpiry().get().path("expiry_seconds").asLong());
  }

  @ParameterizedTest
  @MethodSource("builderCases")
  void withExpiry_subSecond_clampsToOneSecond(BuilderCase builder) {
    ObjectNode config = builder.withExpiry().apply(Duration.ofMillis(500));

    assertEquals(1, config.path("expiry_seconds").asLong());
  }

  @ParameterizedTest
  @MethodSource("builderCases")
  void withExpiry_zero_setsExpirySecondsToZero(BuilderCase builder) {
    ObjectNode config = builder.withExpiry().apply(Duration.ZERO);

    assertEquals(0, config.path("expiry_seconds").asLong());
  }

  @ParameterizedTest
  @MethodSource("builderCases")
  void withExpiry_negative_throwsIllegalArgumentException(BuilderCase builder) {
    Function<Duration, ObjectNode> withExpiry = builder.withExpiry();

    assertThrows(IllegalArgumentException.class, () -> withExpiry.apply(Duration.ofMillis(-500)));
  }
}
