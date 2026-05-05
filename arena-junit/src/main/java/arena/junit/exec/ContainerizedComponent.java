package arena.junit.exec;
import arena.junit.match.ArenaMatchPiece;
import arena.junit.readiness.ReadinessChecksFfi;

import com.fasterxml.jackson.databind.node.ArrayNode;
import com.fasterxml.jackson.databind.node.ObjectNode;
import java.util.ArrayList;
import java.util.List;

public final class ContainerizedComponent implements ArenaMatchPiece {
  private final ObjectNode config;
  private final List<ReadinessChecksFfi.ReadinessEntry> readiness;

  ContainerizedComponent(ObjectNode config, List<ReadinessChecksFfi.ReadinessEntry> readiness) {
    this.config = config;
    this.readiness = List.copyOf(readiness);
  }

  public String identifier() {
    return config.get("identifier").asText();
  }

  @Override
  public ObjectNode forFfi() {
    ObjectNode d = config.deepCopy();
    ArrayNode rc = ReadinessChecksFfi.forExecutable(readiness);
    if (rc.size() > 0) {
      d.set("readiness_checks", rc);
    }
    return d;
  }

  public List<ReadinessChecksFfi.ReadinessEntry> readinessEntries() {
    return readiness;
  }
}
