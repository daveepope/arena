package arena.examples.readings.playbooks;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.List;

final class ReadingsPlaybookConfig {
  private static final ObjectMapper MAPPER = new ObjectMapper();

  static final String CALIBRATION_VALIDATE_PATH;
  static final String CALIBRATION_DEFAULT;
  static final String CALIBRATION_OUTAGE_MANAGED;
  static final String RESET_VALIDATION_DB;
  static final String LOCALSTACK_SESSION;

  static {
    try {
      String path = findConfigPath();
      if (path == null || path.isEmpty()) {
        throw new IllegalStateException("readings_arena_config.json not found");
      }
      JsonNode root = MAPPER.readTree(Files.readString(Path.of(path)));
      JsonNode pb = root.path("playbook_names");
      CALIBRATION_VALIDATE_PATH = root.path("calibration_validate_path").asText();
      CALIBRATION_DEFAULT = pb.path("calibration_default").asText();
      CALIBRATION_OUTAGE_MANAGED = pb.path("calibration_outage_managed").asText();
      RESET_VALIDATION_DB = pb.path("validation_db_scoped").asText();
      LOCALSTACK_SESSION = pb.path("localstack_session").asText();
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
              "arena/examples/resources/readings_arena_config.json",
              "_main/examples/resources/readings_arena_config.json")) {
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
                "examples/resources/readings_arena_config.json",
                "arena/examples/resources/readings_arena_config.json")) {
          Path p =
              Path.of(runfiles)
                  .resolve(base.isEmpty() ? Path.of(rel) : Path.of(base).resolve(rel));
          if (Files.isRegularFile(p)) {
            return p.toString();
          }
        }
      }
    }
    Path dev = Path.of("examples/resources/readings_arena_config.json");
    if (Files.isRegularFile(dev)) {
      return dev.toAbsolutePath().normalize().toString();
    }
    return "";
  }

  private ReadingsPlaybookConfig() {}
}
