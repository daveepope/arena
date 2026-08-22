package arena.examples.component;

import static org.junit.jupiter.api.Assertions.assertTrue;

import arena.examples.http.ApiClient;
import arena.examples.playbooks.ResetWeatherDbPlaybook;
import arena.junit.Arena;
import arena.junit.Playbook;
import com.fasterxml.jackson.databind.JsonNode;
import org.junit.jupiter.api.Test;

@Arena(ComponentTestSuite.class)
final class WeatherCrudComponentTest {

  @Test
  @Playbook(ResetWeatherDbPlaybook.class)
  void createWeatherReportListsViaHttp() throws Exception {
    ApiClient client = ComponentTestSuite.apiClient();
    long createdId = client.createWeatherReport(1.5, 63.2, 1013.25);
    boolean found = false;
    for (JsonNode row : client.getWeatherReports()) {
      if (row.path("id").asLong() == createdId) {
        found = true;
        assertTrue(row.path("precipitation").asDouble() == 1.5);
        assertTrue(row.path("humidity").asDouble() == 63.2);
        assertTrue(row.path("pressure").asDouble() == 1013.25);
      }
    }
    assertTrue(found, "created weather report not listed: " + createdId);
  }

  @Test
  @Playbook(ResetWeatherDbPlaybook.class)
  void createMultipleWeatherReportsAreListed() throws Exception {
    ApiClient client = ComponentTestSuite.apiClient();
    long id1 = client.createWeatherReport(0, 40, 1000);
    long id2 = client.createWeatherReport(2.2, 80, 990.5);
    boolean found1 = false;
    boolean found2 = false;
    for (JsonNode row : client.getWeatherReports()) {
      long id = row.path("id").asLong();
      found1 |= id == id1;
      found2 |= id == id2;
    }
    assertTrue(found1, "first weather report not listed: " + id1);
    assertTrue(found2, "second weather report not listed: " + id2);
  }
}
