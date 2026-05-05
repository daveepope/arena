package arena.junit.match;
import arena.junit.playbook.ArenaPlaybookRegistration;

import com.fasterxml.jackson.databind.node.ObjectNode;

public final class RegisteredPlaybook {
  private final ArenaPlaybookRegistration inner;
  private final boolean execOnDependencyStart;

  public RegisteredPlaybook(ArenaPlaybookRegistration inner, boolean execOnDependencyStart) {
    this.inner = inner;
    this.execOnDependencyStart = execOnDependencyStart;
  }

  public ObjectNode forFfi() {
    ObjectNode n = inner.forRegisteredFfi().deepCopy();
    n.put("exec_on_dependency_start", execOnDependencyStart);
    return n;
  }
}
