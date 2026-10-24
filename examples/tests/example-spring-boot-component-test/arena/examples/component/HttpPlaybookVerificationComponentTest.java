package arena.examples.component;

import static org.junit.jupiter.api.Assertions.assertThrows;

import arena.examples.playbooks.CalibrationApiErrorPathPlaybook;
import arena.junit.Arena;
import arena.junit.Playbook;
import arena.junit.ffi.ArenaBindingError;
import arena.junit.playbook.ActiveHttpPlaybook;
import org.junit.jupiter.api.Test;

@Arena(JavaSpringBootComponentTestSuite.class)
final class HttpPlaybookVerificationComponentTest {

  @Test
  @Playbook(CalibrationApiErrorPathPlaybook.class)
  void httpPlaybookVerifyAtLeastSucceedsWithTraffic(ActiveHttpPlaybook activeHttpPlaybook)
      throws Exception {
    JavaSpringBootComponentTestSuite.apiClient()
        .postReadingRaw("Verify At Least", 3, null, JavaSpringBootComponentTestSuite.readingsDeviceId);
    activeHttpPlaybook.verifyAtLeast("POST", JavaSpringBootComponentTestSuite.CALIBRATION_VALIDATE_PATH, 1);
  }

  @Test
  @Playbook(CalibrationApiErrorPathPlaybook.class)
  void httpPlaybookVerifyCountMismatchRaises(ActiveHttpPlaybook activeHttpPlaybook) {
    assertThrows(
        ArenaBindingError.class,
        () -> activeHttpPlaybook.verify("POST", JavaSpringBootComponentTestSuite.CALIBRATION_VALIDATE_PATH, 1));
  }
}
