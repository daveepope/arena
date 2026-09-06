package arena.junit.dep;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertThrows;

import arena.junit.exec.ContainerizedComponentBuilder;
import com.fasterxml.jackson.databind.node.ObjectNode;
import java.time.Duration;
import org.junit.jupiter.api.Test;

final class ExpiryOverrideUnitTest {

  @Test
  void build_withoutExpiryOverride_omitsExpirySeconds() {
    ObjectNode config = new PostgresDependencyBuilder("orders").build().forFfi();

    assertFalse(config.has("expiry_seconds"));
  }

  @Test
  void withExpiry_thirtySeconds_setsExpirySeconds() {
    ObjectNode config =
        new PostgresDependencyBuilder("orders")
            .withExpiry(Duration.ofSeconds(30))
            .build()
            .forFfi();

    assertEquals(30, config.path("expiry_seconds").asLong());
  }

  @Test
  void withoutExpiry_called_setsExpirySecondsToZero() {
    ObjectNode config =
        new PostgresDependencyBuilder("orders").withoutExpiry().build().forFfi();

    assertEquals(0, config.path("expiry_seconds").asLong());
  }

  @Test
  void withExpiry_containerizedComponent_setsExpirySeconds() {
    ObjectNode config =
        new ContainerizedComponentBuilder("web", "Containerfile")
            .withExpiry(Duration.ofSeconds(45))
            .build()
            .forFfi();

    assertEquals(45, config.path("expiry_seconds").asLong());
  }

  @Test
  void withExpiry_subSecond_clampsToOneSecond() {
    ObjectNode config =
        new PostgresDependencyBuilder("orders")
            .withExpiry(Duration.ofMillis(500))
            .build()
            .forFfi();

    assertEquals(1, config.path("expiry_seconds").asLong());
  }

  @Test
  void withExpiry_negative_throwsIllegalArgumentException() {
    assertThrows(
        IllegalArgumentException.class,
        () -> new PostgresDependencyBuilder("orders").withExpiry(Duration.ofMillis(-500)));
  }

  @Test
  void withExpiry_zero_setsExpirySecondsToZero() {
    ObjectNode config =
        new PostgresDependencyBuilder("orders").withExpiry(Duration.ZERO).build().forFfi();

    assertEquals(0, config.path("expiry_seconds").asLong());
  }
}
