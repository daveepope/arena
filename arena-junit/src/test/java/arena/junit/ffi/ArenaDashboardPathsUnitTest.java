package arena.junit.ffi;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.util.List;
import org.junit.jupiter.api.Test;

final class ArenaDashboardPathsUnitTest {

  @Test
  void dashboardExecutableNameCurrentPlatformMatchesLauncherNaming() {
    List<String> known =
        List.of(
            "arena-dashboard-server-linux",
            "arena-dashboard-server-macos",
            "arena-dashboard-server.exe");

    assertTrue(known.contains(ArenaPaths.dashboardExecutableName()));
  }

  @Test
  void dashboardResourceDirCurrentPlatformMatchesClassifierLayout() {
    List<String> known =
        List.of("linux-x86-64", "darwin-aarch64", "darwin-x86-64", "windows-x86-64");

    assertTrue(known.contains(ArenaPaths.dashboardResourceDir()));
  }

  @Test
  void resolveDashboardSearchDirResourceAbsentReturnsEmpty() {
    String resolved = ArenaPaths.resolveDashboardSearchDir();

    if (ArenaPaths.class.getClassLoader().getResource(
            ArenaPaths.dashboardResourceDir() + "/" + ArenaPaths.dashboardExecutableName())
        == null) {
      assertEquals("", resolved);
    } else {
      assertTrue(resolved.contains(".arena"));
    }
  }
}
