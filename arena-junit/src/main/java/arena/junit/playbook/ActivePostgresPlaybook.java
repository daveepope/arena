package arena.junit.playbook;

import arena.junit.ffi.ArenaBindings;
import arena.junit.ffi.ArenaBindingError;
import arena.junit.support.ArenaJson;

import com.fasterxml.jackson.databind.node.ObjectNode;
import com.sun.jna.Pointer;

public class ActivePostgresPlaybook extends ActivePlaybook {
  protected ActivePostgresPlaybook(Pointer handle) {
    super(handle);
  }

  public void verify(String query, int expectedValue) {
    ObjectNode spec = ArenaJson.object();
    spec.put("query", query);
    spec.put("expected_value", expectedValue);
    try {
      ArenaBindings.postgresPlaybookVerify(handle(), ArenaJson.MAPPER.writeValueAsString(spec));
    } catch (ArenaBindingError e) {
      noteBodyFailure();
      throw e;
    } catch (Exception e) {
      throw new ArenaBindingError(e.getMessage());
    }
  }
}
