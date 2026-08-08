package arena.examples.playbooks;

import arena.junit.playbook.HttpPlaybookBuilder;
import arena.junit.playbook.HttpResponse;
import arena.junit.playbook.ManagedHttpPlaybook;
import java.util.Map;

public final class CalibrationApiFlakyPlaybook extends ManagedHttpPlaybook {
  private static final String IDENTIFIER = "example-api-calibration-api-flaky-path";
  private static final String VALIDATE_PATH = "/api/v1/validate";

  public CalibrationApiFlakyPlaybook(String dependencyIdentifier) {
    super(
        IDENTIFIER,
        dependencyIdentifier,
        new HttpPlaybookBuilder(dependencyIdentifier)
            .post(VALIDATE_PATH)
            .willReturn(HttpResponse.serverError())
            .thenReturn(HttpResponse.status(503))
            .thenReturn(HttpResponse.okJson(Map.of("valid", true))));
  }
}
