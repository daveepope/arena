package arena.examples.component;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import arena.examples.http.ApiClient;
import arena.examples.playbooks.CalibrationApiErrorPathPlaybook;
import arena.examples.playbooks.CalibrationApiFlakyPlaybook;
import arena.examples.playbooks.ResetValidationDbPlaybook;
import arena.junit.Arena;
import arena.junit.Playbook;
import com.fasterxml.jackson.databind.JsonNode;
import java.net.http.HttpResponse;
import org.junit.jupiter.api.Test;

@Arena(JavaSpringBootComponentTestSuite.class)
final class CalibrationOutageComponentTest {

  @Test
  @Playbook(CalibrationApiErrorPathPlaybook.class)
  @Playbook(ResetValidationDbPlaybook.class)
  void postReadingReturns500WhenCalibrationOutagePlaybookActive() throws Exception {
    HttpResponse<String> response =
        JavaSpringBootComponentTestSuite.apiClient()
            .postReadingRaw("Outage Test User", 99, null, JavaSpringBootComponentTestSuite.readingsDeviceId);
    assertEquals(500, response.statusCode(), response.body());
  }

  @Test
  @Playbook(ResetValidationDbPlaybook.class)
  void postReadingSucceedsAfterOutagePlaybookScope() throws Exception {
    int recoveredId =
        JavaSpringBootComponentTestSuite.apiClient()
            .createReading(
                "Recovery Test User", 17, "post-outage", JavaSpringBootComponentTestSuite.readingsDeviceId);
    JsonNode found = JavaSpringBootComponentTestSuite.apiClient().findReadingById(recoveredId);
    assertEquals("Recovery Test User", found.path("user_name").asText());
    assertEquals(17, found.path("value").asInt());
  }

  @Test
  @Playbook(CalibrationApiErrorPathPlaybook.class)
  @Playbook(ResetValidationDbPlaybook.class)
  void postReadingReturns500UnderStackedPlaybooks() throws Exception {
    HttpResponse<String> response =
        JavaSpringBootComponentTestSuite.apiClient()
            .postReadingRaw("Stack Outage", 1, null, JavaSpringBootComponentTestSuite.readingsDeviceId);
    assertEquals(500, response.statusCode(), response.body());
  }

  @Test
  @Playbook(CalibrationApiFlakyPlaybook.class)
  @Playbook(ResetValidationDbPlaybook.class)
  void postReadingSucceedsAfterCalibrationFlakySequence() throws Exception {
    ApiClient client = JavaSpringBootComponentTestSuite.apiClient();
    assertEquals(
        500,
        client.postReadingRaw("Flaky 1", 1, null, JavaSpringBootComponentTestSuite.readingsDeviceId).statusCode());
    assertEquals(
        500,
        client.postReadingRaw("Flaky 2", 2, null, JavaSpringBootComponentTestSuite.readingsDeviceId).statusCode());
    int createdId =
        client.createReading("Flaky 3", 3, "recovered", JavaSpringBootComponentTestSuite.readingsDeviceId);
    assertTrue(client.listReadingIds().contains(createdId));
  }
}
