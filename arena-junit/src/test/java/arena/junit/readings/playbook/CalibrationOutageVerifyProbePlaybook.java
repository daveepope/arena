package arena.junit.readings.playbook;

import arena.junit.playbook.HttpPlaybookBuilder;
import arena.junit.playbook.HttpResponse;
import arena.junit.playbook.ManagedHttpPlaybook;
import arena.junit.readings.fixture.ReadingsArenaConfig;

public final class CalibrationOutageVerifyProbePlaybook extends ManagedHttpPlaybook {
  public static final String IDENTIFIER = "arena-junit-calibration-outage-verify-probe";

  public CalibrationOutageVerifyProbePlaybook(String dependencyIdentifier) {
    super(
        IDENTIFIER,
        dependencyIdentifier,
        new HttpPlaybookBuilder(dependencyIdentifier)
            .post(ReadingsArenaConfig.CALIBRATION_VALIDATE_PATH)
            .willReturn(HttpResponse.serverError()));
  }
}
