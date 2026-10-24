package arena.junit;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNotSame;
import static org.junit.jupiter.api.Assertions.assertThrows;

import arena.junit.ffi.ArenaBindingError;
import arena.junit.ffi.ArenaLogLevel;
import arena.junit.match.Match;
import arena.junit.match.MatchBuilder;
import java.util.List;
import java.util.function.UnaryOperator;
import java.util.stream.Stream;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.MethodSource;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

final class ClosedArenaUnitTest {

  private static final Logger LOG = LoggerFactory.getLogger(ClosedArenaUnitTest.class);

  @Test
  void open_emptyMatches_throwsArenaBindingError() {
    ClosedArena closedArena =
        new ClosedArena(
            "empty-match-arena",
            List.<Match>of(),
            ArenaLogLevel.DEBUG,
            LOG,
            List.of("dependency-one"),
            List.of("component-one"));

    assertThrows(ArenaBindingError.class, closedArena::open);
  }

  @Test
  void open_emptyMatchesWithNullLogIdentifiers_throwsArenaBindingError() {
    ClosedArena closedArena =
        new ClosedArena("empty-match-arena", List.<Match>of(), ArenaLogLevel.INFO, null, null, null);

    assertThrows(ArenaBindingError.class, closedArena::open);
  }

  @Test
  void open_nameAndMatchesOverload_delegatesToInfoLevelDefaults() {
    ClosedArena closedArena = new ClosedArena("empty-match-arena", List.<Match>of());
    assertThrows(ArenaBindingError.class, closedArena::open);
  }

  @Test
  void open_nameMatchesLogLevelOverload_delegatesWithNullLogger() {
    ClosedArena closedArena =
        new ClosedArena("empty-match-arena", List.<Match>of(), ArenaLogLevel.DEBUG);
    assertThrows(ArenaBindingError.class, closedArena::open);
  }

  @Test
  void open_nameMatchesLogLevelLoggerOverload_delegatesWithEmptyLogIds() {
    ClosedArena closedArena =
        new ClosedArena("empty-match-arena", List.<Match>of(), ArenaLogLevel.WARN, LOG);
    assertThrows(ArenaBindingError.class, closedArena::open);
  }

  @Test
  void matchConfigForFfi_noCpuProfiler_omitsTheCpuProfiledComponentsField() {
    Match match = new MatchBuilder("junit-profiled-match").build();
    ClosedArena closedArena = new ClosedArena("junit-profiled-arena", List.of(match));

    assertFalse(closedArena.matchConfigForFfi().has("cpu_profiled_components"));
  }

  @Test
  void withCpuProfiler_singleComponent_serializesThatId() {
    Match match = new MatchBuilder("junit-profiled-match").build();
    ClosedArena closedArena =
        new ClosedArena("junit-profiled-arena", List.of(match))
            .withLifecycleDashboard("http://127.0.0.1:65535")
            .withCpuProfiler("svc")
            .done();

    assertEquals(
        "[\"svc\"]", closedArena.matchConfigForFfi().get("cpu_profiled_components").toString());
  }

  @Test
  void withCpuProfiler_severalComponents_preserveDeclarationOrder() {
    Match match = new MatchBuilder("junit-profiled-match").build();
    ClosedArena closedArena =
        new ClosedArena("junit-profiled-arena", List.of(match))
            .withLifecycleDashboard("http://127.0.0.1:65535")
            .withCpuProfiler("svc")
            .withCpuProfiler("worker")
            .done();

    assertEquals(
        "[\"svc\",\"worker\"]",
        closedArena.matchConfigForFfi().get("cpu_profiled_components").toString());
  }

  @Test
  void withCpuProfileLimitBytes_set_serializesTheLimit() {
    Match match = new MatchBuilder("junit-profiled-match").build();
    ClosedArena closedArena =
        new ClosedArena("junit-profiled-arena", List.of(match))
            .withLifecycleDashboard("http://127.0.0.1:65535")
            .withCpuProfiler("svc")
            .withCpuProfileLimitBytes(1024)
            .done();

    assertEquals(
        1024L, closedArena.matchConfigForFfi().get("cpu_profile_limit_bytes").asLong());
  }

  @Test
  void matchConfigForFfi_noCpuProfileLimit_omitsTheCpuProfileLimitField() {
    Match match = new MatchBuilder("junit-profiled-match").build();
    ClosedArena closedArena =
        new ClosedArena("junit-profiled-arena", List.of(match))
            .withLifecycleDashboard("http://127.0.0.1:65535")
            .withCpuProfiler("svc")
            .done();

    assertFalse(closedArena.matchConfigForFfi().has("cpu_profile_limit_bytes"));
  }

  private static Stream<UnaryOperator<ClosedArena>> dashboardConfigurations() {
    return Stream.of(
        arena -> arena.withLifecycleDashboard().done(),
        arena -> arena.withLifecycleDashboard("http://127.0.0.1:65535").done(),
        arena -> arena.withLifecycleDashboard(1, "/nonexistent-search-dir").done());
  }

  @ParameterizedTest
  @MethodSource("dashboardConfigurations")
  void withLifecycleDashboard_anyOverload_returnsANewArenaStillRejectingEmptyMatches(
      UnaryOperator<ClosedArena> configure) {
    ClosedArena original =
        new ClosedArena("dashboard-arena", List.<Match>of(), ArenaLogLevel.WARN, LOG);

    ClosedArena configured = configure.apply(original);

    assertNotSame(original, configured);
    assertThrows(ArenaBindingError.class, configured::open);
  }
}
