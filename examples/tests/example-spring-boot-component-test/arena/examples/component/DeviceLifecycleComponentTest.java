package arena.examples.component;

import static org.junit.jupiter.api.Assertions.assertEquals;

import arena.examples.http.ApiClient;
import arena.examples.playbooks.ResetReadingsDbPlaybook;
import arena.junit.Arena;
import arena.junit.Playbook;
import java.net.http.HttpResponse;
import java.util.UUID;
import org.junit.jupiter.api.Test;

@Arena(ComponentTestSuite.class)
final class DeviceLifecycleComponentTest {

  @Test
  @Playbook(ResetReadingsDbPlaybook.class)
  void createDeviceRequestTransitionAppliesRequestedState() throws Exception {
    ApiClient client = ComponentTestSuite.apiClient();
    long deviceId = client.createDevice("Smell-O-Scope Mk II");
    assertEquals("OFF", client.getDeviceState(deviceId));

    client.setDeviceState(deviceId, "ON");
    assertEquals("ON", client.getDeviceState(deviceId));

    client.setDeviceState(deviceId, "ERROR");
    assertEquals("ERROR", client.getDeviceState(deviceId));

    client.stopDevice(deviceId);
  }

  @Test
  void getDeviceStateUnknownDeviceReturnsNotFound() throws Exception {
    HttpResponse<String> response = ComponentTestSuite.apiClient().getDeviceStateRaw(999_999_999L);
    assertEquals(404, response.statusCode());
  }

  @Test
  @Playbook(ResetReadingsDbPlaybook.class)
  void createDeviceSendsProvisionedEmailOverStarttls() throws Exception {
    String deviceName = "Mail Probe Device " + UUID.randomUUID().toString().substring(0, 8);
    ComponentTestSuite.apiClient().createDevice(deviceName);
    ComponentTestSuite.waitDeviceProvisionedEmail(deviceName);
  }
}
