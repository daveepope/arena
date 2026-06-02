package arena.junit.readings.playbook;

import arena.junit.playbook.ManagedHttpPlaybook;
import arena.junit.readings.fixture.ReadingsArenaConfig;

import java.util.List;

public final class CalibrationOutagePlaybook extends ManagedHttpPlaybook {
  public CalibrationOutagePlaybook(String dependencyIdentifier) {
    super(
        ReadingsArenaConfig.PLAYBOOK_CALIBRATION_OUTAGE_MANAGED,
        dependencyIdentifier,
        List.of(
            mapping(
                "POST",
                ReadingsArenaConfig.CALIBRATION_VALIDATE_PATH,
                500,
                Expect.calledAtLeast(1))));
  }
}
