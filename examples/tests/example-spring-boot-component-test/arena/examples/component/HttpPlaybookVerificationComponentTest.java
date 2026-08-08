package arena.examples.component;

import static org.junit.jupiter.api.Assertions.assertThrows;

import arena.examples.playbooks.CalibrationApiErrorPathPlaybook;
import arena.junit.Arena;
import arena.junit.Playbook;
import arena.junit.ffi.ArenaBindingError;
import arena.junit.playbook.ActiveHttpPlaybook;
import org.junit.jupiter.api.Test;

@Arena(ComponentTestSuite.class)
final class HttpPlaybookVerificationComponentTest {

  @Test
  @Playbook(CalibrationApiErrorPathPlaybook.class)
  void httpPlaybookVerifyAtLeastSucceedsWithTraffic(ActiveHttpPlaybook activeHttpPlaybook)
      throws Exception {
    ComponentTestSuite.apiClient()
        .postReadingRaw("Verify At Least", 3, null, ComponentTestSuite.readingsDeviceId);
    activeHttpPlaybook.verifyAtLeast("POST", ComponentTestSuite.CALIBRATION_VALIDATE_PATH, 1);
  }

  @Test
  @Playbook(CalibrationApiErrorPathPlaybook.class)
  void httpPlaybookVerifyCountMismatchRaises(ActiveHttpPlaybook activeHttpPlaybook) {
    assertThrows(
        ArenaBindingError.class,
        () -> activeHttpPlaybook.verify("POST", ComponentTestSuite.CALIBRATION_VALIDATE_PATH, 1));
  }
}
