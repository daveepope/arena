package arena.examples.component;

import static org.junit.jupiter.api.Assertions.assertEquals;

import arena.examples.http.ApiClient;
import arena.junit.Arena;
import org.junit.jupiter.api.Test;

@Arena(ChainedComponentTestSuite.class)
final class ChainedDeviceLifecycleComponentTest {

  @Test
  void createDeviceRequestTransitionAppliesRequestedState() throws Exception {
    ApiClient client1 = ChainedComponentTestSuite.apiClient();
    ApiClient client2 = ChainedComponentTestSuite.apiClient2();

    long deviceId = client1.createDevice("Chained Web App Device");
    assertEquals("OFF", client2.getDeviceState(deviceId));

    client2.setDeviceState(deviceId, "ON");
    assertEquals("ON", client1.getDeviceState(deviceId));

    client1.setDeviceState(deviceId, "ERROR");
    assertEquals("ERROR", client2.getDeviceState(deviceId));

    client2.stopDevice(deviceId);
  }
}
