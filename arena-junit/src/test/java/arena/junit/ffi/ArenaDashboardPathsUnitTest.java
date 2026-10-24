package arena.junit.ffi;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.io.File;
import java.util.List;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.CsvSource;

final class ArenaDashboardPathsUnitTest {

  private String originalOsName;
  private String originalOsArch;

  @BeforeEach
  void capturePlatformProperties() {
    originalOsName = System.getProperty("os.name", "");
    originalOsArch = System.getProperty("os.arch", "");
  }

  @AfterEach
  void restorePlatformProperties() {
    System.setProperty("os.name", originalOsName);
    System.setProperty("os.arch", originalOsArch);
  }

  @ParameterizedTest
  @CsvSource({
    "Windows 11, amd64, arena-dashboard-server.exe, windows-x86-64",
    "Mac OS X, aarch64, arena-dashboard-server-macos, darwin-aarch64",
    "Mac OS X, arm64, arena-dashboard-server-macos, darwin-aarch64",
    "Mac OS X, x86_64, arena-dashboard-server-macos, darwin-x86-64",
    "Linux, amd64, arena-dashboard-server-linux, linux-x86-64",
    "Linux, aarch64, arena-dashboard-server-linux, linux-x86-64",
  })
  void dashboardPathsPlatformReturnsExecutableNameAndResourceDir(
      String osName, String osArch, String expectedExecutable, String expectedResourceDir) {
    System.setProperty("os.name", osName);
    System.setProperty("os.arch", osArch);

    assertEquals(expectedExecutable, ArenaPaths.dashboardExecutableName());
    assertEquals(expectedResourceDir, ArenaPaths.dashboardResourceDir());
  }

  @ParameterizedTest
  @CsvSource({"Windows 11", "Mac OS X", "Linux"})
  void resolveArenaSharedLibraryPlatformReturnsEmptyOrAnExistingFile(String osName) {
    System.setProperty("os.name", osName);

    String resolved = ArenaPaths.resolveArenaSharedLibrary();

    assertTrue(resolved.isEmpty() || new File(resolved).isFile(), resolved);
  }

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
