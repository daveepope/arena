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

  static String findSchema(String filename) throws Exception {
    return findRunfile(
        "arena/examples/resources/" + filename,
        "_main/examples/resources/" + filename,
        "examples/resources/" + filename);
  }

  static String findAxumBinary() throws Exception {
    return findRunfile(
        "arena/examples/example-readings-axum-web-app",
        "_main/examples/example-readings-axum-web-app",
        "examples/example-readings-axum-web-app");
  }
}
