package arena.junit.readings.playbook;

import arena.junit.playbook.ManagedHttpPlaybook;
import arena.junit.readings.fixture.ReadingsArenaConfig;

import java.util.List;
import java.util.Map;

public final class ReadingsCalibrationDefaultPlaybook extends ManagedHttpPlaybook {
  public ReadingsCalibrationDefaultPlaybook(String dependencyIdentifier) {
    super(
        ReadingsArenaConfig.PLAYBOOK_CALIBRATION_DEFAULT,
        dependencyIdentifier,
        List.of(
            mapping(
                "POST",
                ReadingsArenaConfig.CALIBRATION_VALIDATE_PATH,
                200,
                Map.of("valid", true),
                Expect.calledAtLeast(1))));
  }
}
