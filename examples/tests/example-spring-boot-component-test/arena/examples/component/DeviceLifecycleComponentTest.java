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
    ApiClient client1 = ComponentTestSuite.apiClient();
    ApiClient client2 = ComponentTestSuite.apiClient2();

    long deviceId = client1.createDevice("Smell-O-Scope Mk II");
    assertEquals("OFF", client2.getDeviceState(deviceId));

    client2.setDeviceState(deviceId, "ON");
    assertEquals("ON", client1.getDeviceState(deviceId));

    client1.setDeviceState(deviceId, "ERROR");
    assertEquals("ERROR", client2.getDeviceState(deviceId));

    client2.stopDevice(deviceId);
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
