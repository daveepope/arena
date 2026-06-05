package arena.junit.readings.playbook;

import arena.junit.playbook.HttpPlaybookBuilder;
import arena.junit.playbook.HttpResponse;
import arena.junit.playbook.ManagedHttpPlaybook;
import arena.junit.readings.fixture.ReadingsArenaConfig;

public final class CalibrationOutagePlaybook extends ManagedHttpPlaybook {
  public CalibrationOutagePlaybook(String dependencyIdentifier) {
    super(
        ReadingsArenaConfig.PLAYBOOK_CALIBRATION_OUTAGE_MANAGED,
        dependencyIdentifier,
        new HttpPlaybookBuilder(dependencyIdentifier)
            .post(ReadingsArenaConfig.CALIBRATION_VALIDATE_PATH)
            .willReturn(HttpResponse.serverError())
            .expectCalledAtLeast(1));
  }
}
