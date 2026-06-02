package arena.junit.playbook;

import arena.junit.ffi.ArenaBindings;
import arena.junit.ffi.ArenaBindingError;
import arena.junit.support.ArenaJson;

import com.fasterxml.jackson.databind.node.ObjectNode;
import com.sun.jna.Pointer;

public class ActiveHttpPlaybook extends ActivePlaybook {
  protected ActiveHttpPlaybook(Pointer handle) {
    super(handle);
  }

  public void verify(String method, String urlPath, int expectedCount) {
    ObjectNode spec = ArenaJson.object();
    spec.put("method", method.toUpperCase());
    spec.put("url_path", urlPath);
    spec.put("expected_count", expectedCount);
    try {
      ArenaBindings.httpPlaybookVerify(handle(), ArenaJson.MAPPER.writeValueAsString(spec));
    } catch (ArenaBindingError e) {
      noteBodyFailure();
      throw e;
    } catch (Exception e) {
      throw new ArenaBindingError(e.getMessage());
    }
  }

  public void verifyAtLeast(String method, String urlPath, int minimumCount) {
    ObjectNode spec = ArenaJson.object();
    spec.put("method", method.toUpperCase());
    spec.put("url_path", urlPath);
    spec.put("minimum_count", minimumCount);
    try {
      ArenaBindings.httpPlaybookVerify(handle(), ArenaJson.MAPPER.writeValueAsString(spec));
    } catch (ArenaBindingError e) {
      noteBodyFailure();
      throw e;
    } catch (Exception e) {
      throw new ArenaBindingError(e.getMessage());
    }
  }
}
