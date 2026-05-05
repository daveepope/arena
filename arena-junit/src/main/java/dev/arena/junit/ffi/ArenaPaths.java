package dev.arena.junit.ffi;
import java.io.File;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Locale;

public final class ArenaPaths {
  private ArenaPaths() {}

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
