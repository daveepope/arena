package arena.examples.playbooks;

import arena.junit.playbook.ManagedHttpPlaybook;
import java.util.List;
import java.util.Map;

public final class CalibrationApiHappyPathPlaybook extends ManagedHttpPlaybook {
  public CalibrationApiHappyPathPlaybook(String dependencyIdentifier) {
    super(
        PlaybookConfig.CALIBRATION_API_HAPPY_PATH,
        dependencyIdentifier,
        List.of(
            mapping(
                "POST",
                PlaybookConfig.CALIBRATION_VALIDATE_PATH,
                200,
                Map.of("valid", true),
                Expect.calledAtLeast(1))));
  }
}
