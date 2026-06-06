package arena.examples.playbooks;

import arena.junit.playbook.HttpPlaybookBuilder;
import arena.junit.playbook.HttpResponse;
import arena.junit.playbook.ManagedHttpPlaybook;
import java.util.Map;

public final class CalibrationApiFlakyPlaybook extends ManagedHttpPlaybook {
  public CalibrationApiFlakyPlaybook(String dependencyIdentifier) {
    super(
        PlaybookConfig.CALIBRATION_API_FLAKY_PATH,
        dependencyIdentifier,
        new HttpPlaybookBuilder(dependencyIdentifier)
            .post(PlaybookConfig.CALIBRATION_VALIDATE_PATH)
            .willReturn(HttpResponse.serverError())
            .thenReturn(HttpResponse.status(503))
            .thenReturn(HttpResponse.okJson(Map.of("valid", true))));
  }
}
