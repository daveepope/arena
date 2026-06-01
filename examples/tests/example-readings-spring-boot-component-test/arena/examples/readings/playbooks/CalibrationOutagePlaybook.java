package arena.examples.readings.playbooks;

import arena.junit.playbook.ManagedHttpPlaybook;
import java.util.List;

public final class CalibrationOutagePlaybook extends ManagedHttpPlaybook {
  public CalibrationOutagePlaybook(String dependencyIdentifier) {
    super(
        ReadingsPlaybookConfig.CALIBRATION_OUTAGE_MANAGED,
        dependencyIdentifier,
        List.of(
            mapping(
                "POST",
                ReadingsPlaybookConfig.CALIBRATION_VALIDATE_PATH,
                500,
                Expect.calledAtLeast(1))));
  }
}
