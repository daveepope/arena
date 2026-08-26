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

@Arena(ComponentTestSuite.class)
final class CalibrationOutageComponentTest {

  @Test
  @Playbook(CalibrationApiErrorPathPlaybook.class)
  @Playbook(ResetValidationDbPlaybook.class)
  void postReadingReturns500WhenCalibrationOutagePlaybookActive() throws Exception {
    HttpResponse<String> response =
        ComponentTestSuite.apiClient().postReadingRaw("Outage Test User", 99, null);
    assertEquals(500, response.statusCode(), response.body());
  }

  @Test
  @Playbook(ResetValidationDbPlaybook.class)
  void postReadingSucceedsAfterOutagePlaybookScope() throws Exception {
    int recoveredId =
        ComponentTestSuite.apiClient().createReading("Recovery Test User", 17, "post-outage");
    JsonNode found = ComponentTestSuite.apiClient().findReadingById(recoveredId);
    assertEquals("Recovery Test User", found.path("user_name").asText());
    assertEquals(17, found.path("value").asInt());
  }

  @Test
  @Playbook(CalibrationApiErrorPathPlaybook.class)
  @Playbook(ResetValidationDbPlaybook.class)
  void postReadingReturns500UnderStackedPlaybooks() throws Exception {
    HttpResponse<String> response =
        ComponentTestSuite.apiClient().postReadingRaw("Stack Outage", 1, null);
    assertEquals(500, response.statusCode(), response.body());
  }

  @Test
  @Playbook(CalibrationApiFlakyPlaybook.class)
  @Playbook(ResetValidationDbPlaybook.class)
  void postReadingSucceedsAfterCalibrationFlakySequence() throws Exception {
    ApiClient client = ComponentTestSuite.apiClient();
    assertEquals(500, client.postReadingRaw("Flaky 1", 1, null).statusCode());
    assertEquals(500, client.postReadingRaw("Flaky 2", 2, null).statusCode());
    int createdId = client.createReading("Flaky 3", 3, "recovered");
    assertTrue(client.listReadingIds().contains(createdId));
  }
}
