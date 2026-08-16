package arena.examples.component;

import java.nio.file.Files;
import java.nio.file.Path;
import java.util.List;

final class Runfiles {

  private Runfiles() {}

  static String findRunfile(String... candidates) throws Exception {
    try {
      Class<?> rf = Class.forName("com.google.devtools.build.runfiles.Runfiles");
      Object r = rf.getMethod("create").invoke(null);
      for (String c : candidates) {
        String p = (String) rf.getMethod("rlocation", String.class).invoke(r, c);
        if (p != null && !p.isEmpty() && Files.isRegularFile(Path.of(p))) {
          return p;
        }
      }
    } catch (ClassNotFoundException ignored) {
    }
    String runfiles = System.getenv("RUNFILES_DIR");
    if (runfiles != null) {
      for (String base : List.of("_main", "arena", "")) {
        for (String c : candidates) {
          Path p =
              Path.of(runfiles)
                  .resolve(base.isEmpty() ? Path.of(c) : Path.of(base).resolve(c));
          if (Files.isRegularFile(p)) {
            return p.toString();
          }
        }
      }
    }
    for (String c : candidates) {
      Path p = Path.of(c);
      if (Files.isRegularFile(p)) {
        return p.toAbsolutePath().normalize().toString();
      }
    }
    return "";
  }

  // Like findRunfile, but for executable binaries: the real file may carry a
  // platform-specific extension the candidates don't include (e.g. `.exe` on
  // Windows), so on a miss this hands back a best-guess path unconditionally
  // instead of failing here, and lets the caller's own executable resolution
  // (which already knows how to try the platform's extension) verify it and
  // report a clear error if it's genuinely missing.
  static String findExecutableRunfile(String... candidates) throws Exception {
    String found = findRunfile(candidates);
    if (!found.isEmpty()) {
      return found;
    }
    String runfiles = System.getenv("RUNFILES_DIR");
    if (runfiles != null && candidates.length > 0) {
      String bestGuess = candidates.length > 1 ? candidates[1] : candidates[0];
      return Path.of(runfiles).resolve(bestGuess).toString();
    }
    return "";
  }

  static String findSchema(String filename) throws Exception {
    return findRunfile(
        "arena/examples/resources/" + filename,
        "_main/examples/resources/" + filename,
        "examples/resources/" + filename);
  }

  static String findAxumBinary() throws Exception {
    return findExecutableRunfile(
        "arena/examples/example-readings-axum-web-app",
        "_main/examples/example-readings-axum-web-app",
        "examples/example-readings-axum-web-app");
  }
}
