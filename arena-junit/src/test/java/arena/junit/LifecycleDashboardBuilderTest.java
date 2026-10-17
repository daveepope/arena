package arena.junit;

import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNotSame;
import static org.junit.jupiter.api.Assertions.assertTrue;

import arena.junit.match.MatchBuilder;
import java.util.List;
import org.junit.jupiter.api.Test;

class LifecycleDashboardBuilderTest {

  private static ClosedArena unprofiled() {
    return new ClosedArena(
        "junit-dashboard-builder-arena",
        List.of(new MatchBuilder("junit-dashboard-builder-match").build()));
  }

  @Test
  void done_afterWithCpuProfiler_returnsAnArenaCarryingTheProfiler() {
    ClosedArena configured =
        unprofiled()
            .withLifecycleDashboard("http://127.0.0.1:65535")
            .withCpuProfiler("svc")
            .done();

    assertTrue(configured.matchConfigForFfi().has("cpu_profiled_components"));
  }

  @Test
  void withCpuProfiler_called_leavesTheOriginatingArenaUnprofiled() {
    ClosedArena original = unprofiled();

    ClosedArena configured =
        original.withLifecycleDashboard("http://127.0.0.1:65535").withCpuProfiler("svc").done();

    assertNotSame(original, configured);
    assertFalse(original.matchConfigForFfi().has("cpu_profiled_components"));
  }
}
