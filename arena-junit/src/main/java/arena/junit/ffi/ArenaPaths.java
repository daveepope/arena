package arena.junit.ffi;
import com.sun.jna.Library;
import com.sun.jna.Native;
import java.io.File;
import java.io.InputStream;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.StandardCopyOption;
import java.security.MessageDigest;
import java.util.Locale;
import java.util.Map;

final class ArenaPaths {
  private ArenaPaths() {}

  static <T extends Library> T loadFromClasspath(Class<T> libraryInterface) {
    try {
      Map<String, Object> options = Map.of(Library.OPTION_STRING_ENCODING, "UTF-8");
      return Native.load("arena_ffi_shared", libraryInterface, options);
    } catch (LinkageError e) {
      return null;
    }
  }

  static String dashboardExecutableName() {
    String os = System.getProperty("os.name", "").toLowerCase(Locale.ROOT);
    if (os.contains("windows")) {
      return "arena-dashboard-server.exe";
    }
    if (os.contains("mac")) {
      return "arena-dashboard-server-macos";
    }
    return "arena-dashboard-server-linux";
  }

  static String dashboardResourceDir() {
    String os = System.getProperty("os.name", "").toLowerCase(Locale.ROOT);
    String arch = System.getProperty("os.arch", "").toLowerCase(Locale.ROOT);
    boolean aarch64 = arch.contains("aarch64") || arch.contains("arm64");
    if (os.contains("windows")) {
      return "windows-x86-64";
    }
    if (os.contains("mac")) {
      return aarch64 ? "darwin-aarch64" : "darwin-x86-64";
    }
    return "linux-x86-64";
  }

  static String resolveDashboardSearchDir() {
    String resource = dashboardResourceDir() + "/" + dashboardExecutableName();
    try (InputStream in = ArenaPaths.class.getClassLoader().getResourceAsStream(resource)) {
      if (in == null) {
        return "";
      }
      byte[] bytes = in.readAllBytes();
      Path dir =
          Path.of(System.getProperty("user.home", "."), ".arena", "dashboard", digest(bytes));
      Files.createDirectories(dir);
      Path executable = dir.resolve(dashboardExecutableName());
      if (!Files.isRegularFile(executable) || Files.size(executable) != bytes.length) {
        Path staged = dir.resolve(dashboardExecutableName() + ".partial");
        Files.write(staged, bytes);
        staged.toFile().setExecutable(true, true);
        Files.move(staged, executable, StandardCopyOption.REPLACE_EXISTING);
      }
      executable.toFile().setExecutable(true, true);
      return dir.toAbsolutePath().toString();
    } catch (Exception e) {
      return "";
    }
  }

  private static String digest(byte[] bytes) throws Exception {
    byte[] hash = MessageDigest.getInstance("SHA-256").digest(bytes);
    StringBuilder out = new StringBuilder();
    for (int i = 0; i < 6; i++) {
      out.append(String.format("%02x", hash[i]));
    }
    return out.toString();
  }

  public static String resolveArenaSharedLibrary() {
    String env = System.getenv("ARENA_FFI_LIB");
    if (env != null && !env.isEmpty()) {
      File f = new File(env);
      if (f.isFile()) {
        return f.getAbsolutePath();
      }
    }
    String runfiles = System.getenv("RUNFILES_DIR");
    if (runfiles != null) {
      String os = System.getProperty("os.name", "").toLowerCase(Locale.ROOT);
      String[] names;
      if (os.contains("mac")) {
        names = new String[] {"libarena_ffi_shared.dylib", "libarena_ffi.dylib"};
      } else if (os.contains("windows")) {
        names = new String[] {"arena_ffi_shared.dll", "arena_ffi.dll"};
      } else {
        names = new String[] {"libarena_ffi_shared.so", "libarena_ffi.so"};
      }
      String[] bases = new String[] {"_main/arena-ffi", "arena-ffi", "arena/arena-ffi"};
      for (String base : bases) {
        for (String n : names) {
          Path p = Path.of(runfiles, base, n);
          if (Files.isRegularFile(p)) {
            return p.toAbsolutePath().toString();
          }
        }
      }
    }
    return "";
  }
}
