package arena.examples.component;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import arena.examples.http.ApiClient;
import arena.examples.playbooks.CalibrationApiErrorPathPlaybook;
import arena.examples.playbooks.CalibrationApiFlakyPlaybook;
import arena.examples.playbooks.ResetValidationDbPlaybook;
import arena.junit.Playbook;
import arena.junit.ffi.ArenaBindingError;
import arena.junit.playbook.ActiveHttpPlaybook;
import com.fasterxml.jackson.databind.JsonNode;
import java.net.http.HttpResponse;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.extension.RegisterExtension;

public final class ComponentTest {

  @RegisterExtension
  static final ArenaFixture arena = new ArenaFixture();

  @Test
  @Playbook(ResetValidationDbPlaybook.class)
  void createReadingPublishesEventAndListsViaHttp() throws Exception {
    int createdId = apiClient().createReading("Readings API User", 77, "sqs happy path");
    JsonNode detail = waitReadingCreatedOnQueue(createdId);
    assertEquals(createdId, detail.path("id").asInt());
    assertEquals("Readings API User", detail.path("user_name").asText());
    assertEquals(77, detail.path("value").asInt());
    assertEquals("sqs happy path", detail.path("comment").asText());

    JsonNode found = apiClient().findReadingById(createdId);
    assertEquals("Readings API User", found.path("user_name").asText());
    assertEquals(77, found.path("value").asInt());
  }

  @Test
  @Playbook(ResetValidationDbPlaybook.class)
  void createMultipleReadingsAreListed() throws Exception {
    ApiClient client = apiClient();
    int id1 = client.createReading("Bending", 1, "");
    int id2 = client.createReading("joe", 2, "We're going to need a bigger ship");
    assertTrue(client.listReadingIds().contains(id1));
    assertTrue(client.listReadingIds().contains(id2));
  }

  @Test
  @Playbook(CalibrationApiErrorPathPlaybook.class)
  @Playbook(ResetValidationDbPlaybook.class)
  void postReadingReturns500WhenCalibrationOutagePlaybookActive() throws Exception {
    HttpResponse<String> response = apiClient().postReadingRaw("Outage Test User", 99, null);
    assertEquals(500, response.statusCode(), response.body());
  }

  @Test
  @Playbook(ResetValidationDbPlaybook.class)
  void postReadingSucceedsAfterOutagePlaybookScope() throws Exception {
    int recoveredId = apiClient().createReading("Recovery Test User", 17, "post-outage");
    JsonNode found = apiClient().findReadingById(recoveredId);
    assertEquals("Recovery Test User", found.path("user_name").asText());
    assertEquals(17, found.path("value").asInt());
  }

  @Test
  @Playbook(ResetValidationDbPlaybook.class)
  void createReadingWithValidationDbScopedPlaybook() throws Exception {
    int createdId = apiClient().createReading("Validation DB Scoped", 7, "mssql scope");
    assertTrue(apiClient().listReadingIds().contains(createdId));
  }

  @Test
  @Playbook(CalibrationApiErrorPathPlaybook.class)
  @Playbook(ResetValidationDbPlaybook.class)
  void postReadingReturns500UnderStackedPlaybooks() throws Exception {
    HttpResponse<String> response = apiClient().postReadingRaw("Stack Outage", 1, null);
    assertEquals(500, response.statusCode(), response.body());
  }

  @Test
  @Playbook(CalibrationApiFlakyPlaybook.class)
  @Playbook(ResetValidationDbPlaybook.class)
  void postReadingSucceedsAfterCalibrationFlakySequence() throws Exception {
    ApiClient client = apiClient();
    assertEquals(500, client.postReadingRaw("Flaky 1", 1, null).statusCode());
    assertEquals(500, client.postReadingRaw("Flaky 2", 2, null).statusCode());
    int createdId = client.createReading("Flaky 3", 3, "recovered");
    assertTrue(client.listReadingIds().contains(createdId));
  }

  @Test
  @Playbook(CalibrationApiErrorPathPlaybook.class)
  void httpPlaybookVerifyAtLeastSucceedsWithTraffic(ActiveHttpPlaybook activeHttpPlaybook)
      throws Exception {
    apiClient().postReadingRaw("Verify At Least", 3, null);
    activeHttpPlaybook.verifyAtLeast("POST", ArenaFixture.CALIBRATION_VALIDATE_PATH, 1);
  }

  @Test
  @Playbook(CalibrationApiErrorPathPlaybook.class)
  void httpPlaybookVerifyCountMismatchRaises(ActiveHttpPlaybook activeHttpPlaybook) {
    assertThrows(
        ArenaBindingError.class,
        () -> activeHttpPlaybook.verify("POST", ArenaFixture.CALIBRATION_VALIDATE_PATH, 1));
  }

  private static ApiClient apiClient() {
    return new ApiClient(
        "http://127.0.0.1:" + arena.webAppPort(), arena.accessToken(), ArenaFixture.MAPPER);
  }

  private static JsonNode waitReadingCreatedOnQueue(int expectedId) throws Exception {
    return SqsWait.waitReadingCreatedDetail(
        ArenaFixture.MAPPER, arena.sqsClient(), arena.sqsQueueUrl(), expectedId);
  }
}
