package arena.junit.match;

import arena.junit.playbook.PlaybookRegistration;
import arena.junit.playbook.Playbook;

import com.fasterxml.jackson.databind.node.ObjectNode;

public final class RegisteredPlaybook {
  private final Playbook playbook;
  private final boolean execOnDependencyStart;

  public RegisteredPlaybook(Playbook playbook, boolean execOnDependencyStart) {
    if (execOnDependencyStart && !(playbook instanceof PlaybookRegistration)) {
      throw new IllegalArgumentException(
          "playbook "
              + playbook.getClass().getName()
              + " must implement PlaybookRegistration to be registered with"
              + " execOnDependencyStart=true");
    }
    this.playbook = playbook;
    this.execOnDependencyStart = execOnDependencyStart;
  }

  public Playbook playbook() {
    return playbook;
  }

  public boolean execOnDependencyStart() {
    return execOnDependencyStart;
  }

  public ObjectNode forFfi() {
    if (!(playbook instanceof PlaybookRegistration registration)) {
      return null;
    }
    ObjectNode n = registration.forRegisteredFfi().deepCopy();
    n.put("exec_on_dependency_start", execOnDependencyStart);
    return n;
  }
}
