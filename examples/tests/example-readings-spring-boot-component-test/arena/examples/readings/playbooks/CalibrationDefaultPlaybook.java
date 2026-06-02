package arena.examples.readings.playbooks;

import arena.junit.playbook.ManagedHttpPlaybook;
import java.util.List;
import java.util.Map;

public final class CalibrationDefaultPlaybook extends ManagedHttpPlaybook {
  public CalibrationDefaultPlaybook(String dependencyIdentifier) {
    super(
        ReadingsPlaybookConfig.CALIBRATION_DEFAULT,
        dependencyIdentifier,
        List.of(
            mapping(
                "POST",
                ReadingsPlaybookConfig.CALIBRATION_VALIDATE_PATH,
                200,
                Map.of("valid", true),
                Expect.calledAtLeast(1))));
  }
}
