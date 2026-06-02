package arena.junit.readings.playbook;

import arena.junit.playbook.ManagedHttpPlaybook;
import arena.junit.readings.fixture.ReadingsArenaConfig;

import java.util.List;

public final class CalibrationOutageVerifyProbePlaybook extends ManagedHttpPlaybook {
  public static final String IDENTIFIER = "arena-junit-calibration-outage-verify-probe";

  public CalibrationOutageVerifyProbePlaybook(String dependencyIdentifier) {
    super(
        IDENTIFIER,
        dependencyIdentifier,
        List.of(
            mapping(
                "POST",
                ReadingsArenaConfig.CALIBRATION_VALIDATE_PATH,
                500)));
  }
}
