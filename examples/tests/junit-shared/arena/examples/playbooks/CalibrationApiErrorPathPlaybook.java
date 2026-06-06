package arena.examples.playbooks;

import arena.junit.playbook.ManagedHttpPlaybook;
import java.util.List;

public final class CalibrationApiErrorPathPlaybook extends ManagedHttpPlaybook {
  public CalibrationApiErrorPathPlaybook(String dependencyIdentifier) {
    super(
        PlaybookConfig.CALIBRATION_API_ERROR_PATH,
        dependencyIdentifier,
        List.of(
            mapping(
                "POST",
                PlaybookConfig.CALIBRATION_VALIDATE_PATH,
                500)));
  }
}
