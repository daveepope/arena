package arena.junit.readings.playbook;

import arena.junit.playbook.HttpPlaybookBuilder;
import arena.junit.playbook.HttpResponse;
import arena.junit.playbook.ManagedHttpPlaybook;
import arena.junit.readings.fixture.ReadingsArenaConfig;

import java.util.Map;

public final class CalibrationDefaultPlaybook extends ManagedHttpPlaybook {
  public CalibrationDefaultPlaybook(String dependencyIdentifier) {
    super(
        ReadingsArenaConfig.PLAYBOOK_CALIBRATION_DEFAULT,
        dependencyIdentifier,
        new HttpPlaybookBuilder(dependencyIdentifier)
            .post(ReadingsArenaConfig.CALIBRATION_VALIDATE_PATH)
            .willReturn(HttpResponse.okJson(Map.of("valid", true)))
            .expectCalledAtLeast(1));
  }
}
