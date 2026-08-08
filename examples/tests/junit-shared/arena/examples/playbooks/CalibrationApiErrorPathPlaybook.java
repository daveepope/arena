package arena.examples.playbooks;

import arena.junit.playbook.ManagedHttpPlaybook;
import java.util.List;

public final class CalibrationApiErrorPathPlaybook extends ManagedHttpPlaybook {
  private static final String IDENTIFIER = "example-api-calibration-api-error-path";
  private static final String VALIDATE_PATH = "/api/v1/validate";

  public CalibrationApiErrorPathPlaybook(String dependencyIdentifier) {
    super(IDENTIFIER, dependencyIdentifier, List.of(mapping("POST", VALIDATE_PATH, 500)));
  }
}
