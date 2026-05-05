package arena.junit.readiness;
import arena.junit.support.ArenaJson;

import com.fasterxml.jackson.databind.node.ArrayNode;
import com.fasterxml.jackson.databind.node.ObjectNode;
import java.util.List;

public final class ReadinessChecksFfi {
  private ReadinessChecksFfi() {}

  public static ArrayNode forExecutable(List<ReadinessEntry> checks) {
    ArrayNode out = ArenaJson.array();
    for (ReadinessEntry e : checks) {
      if (e.check() instanceof HttpReadinessCheck) {
        ObjectNode n = ArenaJson.object();
        n.put("kind", "http");
        n.put("target", e.target());
        out.add(n);
      }
    }
    return out;
  }

  public record ReadinessEntry(ReadinessCheck check, String target) {}
}
