package arena.junit.playbook;

import static org.junit.jupiter.api.Assertions.assertDoesNotThrow;
import static org.junit.jupiter.api.Assertions.assertThrows;

import com.sun.jna.Pointer;

import org.junit.jupiter.api.Test;

final class ActivePlaybookUnitTest {

  static final class StubActivePlaybook extends ActivePlaybook {
    StubActivePlaybook(Pointer handle) {
      super(handle);
    }

    void triggerBodyFailure() {
      noteBodyFailure();
    }

    Pointer exposedHandle() {
      return handle();
    }
  }

  @Test
  void handle_nullPointer_throwsIllegalStateException() {
    StubActivePlaybook playbook = new StubActivePlaybook(null);
    assertThrows(IllegalStateException.class, playbook::exposedHandle);
  }

  @Test
  void handle_zeroValuePointer_throwsIllegalStateException() {
    StubActivePlaybook playbook = new StubActivePlaybook(Pointer.NULL);
    assertThrows(IllegalStateException.class, playbook::exposedHandle);
  }

  @Test
  void close_alreadyNullHandle_returnsWithoutThrowing() {
    StubActivePlaybook playbook = new StubActivePlaybook(null);
    assertDoesNotThrow(playbook::close);
  }

  @Test
  void close_zeroValuePointer_returnsWithoutThrowing() {
    StubActivePlaybook playbook = new StubActivePlaybook(Pointer.NULL);
    assertDoesNotThrow(playbook::close);
  }

  @Test
  void close_calledTwice_secondCallIsNoOp() {
    StubActivePlaybook playbook = new StubActivePlaybook(Pointer.NULL);
    playbook.close();
    assertDoesNotThrow(playbook::close);
  }

  @Test
  void noteBodyFailure_thenClose_doesNotThrowOnClosedHandle() {
    StubActivePlaybook playbook = new StubActivePlaybook(Pointer.NULL);
    playbook.triggerBodyFailure();
    assertDoesNotThrow(playbook::close);
  }
}
