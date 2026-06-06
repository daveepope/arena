package arena.examples.playbooks;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.List;

final class PlaybookConfig {
  private static final ObjectMapper MAPPER = new ObjectMapper();

  static final String CALIBRATION_VALIDATE_PATH;
  static final String CALIBRATION_API_HAPPY_PATH;
  static final String CALIBRATION_API_ERROR_PATH;
  static final String CALIBRATION_API_FLAKY_PATH;
  static final String RESET_VALIDATION_DB;
  static final String EVENTS_PURGE;

  static {
    try {
      String path = findConfigPath();
      if (path == null || path.isEmpty()) {
        throw new IllegalStateException("arena_config.json not found");
      }
      JsonNode root = MAPPER.readTree(Files.readString(Path.of(path)));
      JsonNode pb = root.path("playbook_names");
      CALIBRATION_VALIDATE_PATH = root.path("calibration_validate_path").asText();
      CALIBRATION_API_HAPPY_PATH = pb.path("calibration_api_happy_path").asText();
      CALIBRATION_API_ERROR_PATH = pb.path("calibration_api_error_path").asText();
      CALIBRATION_API_FLAKY_PATH = pb.path("calibration_api_flaky_path").asText();
      RESET_VALIDATION_DB = pb.path("validation_db_scoped").asText();
      EVENTS_PURGE = pb.path("events_purge").asText();
    } catch (Exception e) {
      throw new ExceptionInInitializerError(e);
    }
  }

  private static String findConfigPath() throws Exception {
    try {
      Class<?> rf = Class.forName("com.google.devtools.build.runfiles.Runfiles");
      Object r = rf.getMethod("create").invoke(null);
      for (String rel :
          List.of(
              "arena/examples/resources/arena_config.json",
              "_main/examples/resources/arena_config.json")) {
        String p = (String) rf.getMethod("rlocation", String.class).invoke(r, rel);
        if (p != null && !p.isEmpty() && Files.isRegularFile(Path.of(p))) {
          return p;
        }
      }
    } catch (ClassNotFoundException ignored) {
    }
    String runfiles = System.getenv("RUNFILES_DIR");
    if (runfiles != null) {
      for (String base : List.of("_main", "arena", "")) {
        for (String rel :
            List.of(
                "examples/resources/arena_config.json",
                "arena/examples/resources/arena_config.json")) {
          Path p =
              Path.of(runfiles)
                  .resolve(base.isEmpty() ? Path.of(rel) : Path.of(base).resolve(rel));
          if (Files.isRegularFile(p)) {
            return p.toString();
          }
        }
      }
    }
    Path dev = Path.of("examples/resources/arena_config.json");
    if (Files.isRegularFile(dev)) {
      return dev.toAbsolutePath().normalize().toString();
    }
    return "";
  }

  private PlaybookConfig() {}
}
