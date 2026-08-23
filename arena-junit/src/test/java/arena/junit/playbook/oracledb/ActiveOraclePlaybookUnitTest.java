package arena.junit.playbook.oracledb;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;

import arena.junit.ffi.ArenaBindingError;

import com.sun.jna.Pointer;

import org.junit.jupiter.api.Test;

final class ActiveOraclePlaybookUnitTest {

  @Test
  void verify_nullHandle_wrapsClosedStateAsArenaBindingError() {
    ActiveOraclePlaybook playbook = new ActiveOraclePlaybook(null);
    ArenaBindingError error =
        assertThrows(
            ArenaBindingError.class, () -> playbook.verify("select 1 from dual", 1));
    assertEquals("active playbook is already closed", error.getMessage());
  }

  @Test
  void verify_zeroValueHandle_wrapsClosedStateAsArenaBindingError() {
    ActiveOraclePlaybook playbook = new ActiveOraclePlaybook(Pointer.NULL);
    ArenaBindingError error =
        assertThrows(
            ArenaBindingError.class, () -> playbook.verify("select 1 from dual", 1));
    assertEquals("active playbook is already closed", error.getMessage());
  }

  @Test
  void close_zeroValueHandle_returnsWithoutThrowing() {
    ActiveOraclePlaybook playbook = new ActiveOraclePlaybook(Pointer.NULL);
    playbook.close();
  }
}
