package arena.examples.playbooks;

import arena.junit.playbook.ManagedHttpPlaybook;
import java.util.List;
import java.util.Map;

public final class CalibrationApiHappyPathPlaybook extends ManagedHttpPlaybook {
  private static final String IDENTIFIER = "example-api-calibration-api-happy-path";
  private static final String VALIDATE_PATH = "/api/v1/validate";

  public CalibrationApiHappyPathPlaybook(String dependencyIdentifier) {
    super(
        IDENTIFIER,
        dependencyIdentifier,
        List.of(mapping("POST", VALIDATE_PATH, 200, Map.of("valid", true))));
  }
}
