package arena.examples.component;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import arena.examples.http.ApiClient;
import arena.examples.playbooks.ResetValidationDbPlaybook;
import arena.examples.playbooks.SeedValidationReadingPlaybook;
import arena.junit.Arena;
import arena.junit.Playbook;
import com.fasterxml.jackson.databind.JsonNode;
import org.junit.jupiter.api.Test;

@Arena(ComponentTestSuite.class)
final class ReadingsCrudComponentTest {

  @Test
  @Playbook(ResetValidationDbPlaybook.class)
  void createReadingPublishesEventAndListsViaHttp() throws Exception {
    int createdId =
        ComponentTestSuite.apiClient()
            .createReading(
                "Readings API User", 77, "sqs happy path", ComponentTestSuite.readingsDeviceId);
    JsonNode detail = ComponentTestSuite.waitReadingCreatedOnQueue(createdId);
    assertEquals(createdId, detail.path("id").asInt());
    assertEquals("Readings API User", detail.path("user_name").asText());
    assertEquals(77, detail.path("value").asInt());
    assertEquals("sqs happy path", detail.path("comment").asText());

    JsonNode found = ComponentTestSuite.apiClient().findReadingById(createdId);
    assertEquals("Readings API User", found.path("user_name").asText());
    assertEquals(77, found.path("value").asInt());
  }

  @Test
  @Playbook(ResetValidationDbPlaybook.class)
  void createMultipleReadingsAreListed() throws Exception {
    ApiClient client = ComponentTestSuite.apiClient();
    int id1 = client.createReading("Bending", 1, "", ComponentTestSuite.readingsDeviceId);
    int id2 =
        client.createReading(
            "joe", 2, "We're going to need a bigger ship", ComponentTestSuite.readingsDeviceId);
    assertTrue(client.listReadingIds().contains(id1));
    assertTrue(client.listReadingIds().contains(id2));
  }

  @Test
  @Playbook(ResetValidationDbPlaybook.class)
  void createReadingWithValidationDbScopedPlaybook() throws Exception {
    int createdId =
        ComponentTestSuite.apiClient()
            .createReading(
                "Validation DB Scoped", 7, "mssql scope", ComponentTestSuite.readingsDeviceId);
    assertTrue(ComponentTestSuite.apiClient().listReadingIds().contains(createdId));
  }

  @Test
  @Playbook(ResetValidationDbPlaybook.class)
  @Playbook(SeedValidationReadingPlaybook.class)
  void seedValidationReadingPlaybook_rowVisibleBeforeTestBody() throws Exception {
    assertEquals(1, ComponentTestSuite.seededValidationRowCount());
  }
}
