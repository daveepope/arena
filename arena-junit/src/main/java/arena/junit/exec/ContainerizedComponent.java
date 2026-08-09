package arena.junit.exec;
import arena.junit.match.ArenaMatchPiece;
import arena.junit.readiness.ReadinessChecksFfi;
import arena.junit.support.ChildrenFfi;

import com.fasterxml.jackson.databind.node.ArrayNode;
import com.fasterxml.jackson.databind.node.ObjectNode;
import java.util.ArrayList;
import java.util.List;

public final class ContainerizedComponent implements ArenaMatchPiece {
  private final ObjectNode config;
  private final List<ReadinessChecksFfi.ReadinessEntry> readiness;
  private final List<ArenaMatchPiece> children;

  ContainerizedComponent(
      ObjectNode config,
      List<ReadinessChecksFfi.ReadinessEntry> readiness,
      List<ArenaMatchPiece> children) {
    this.config = config;
    this.readiness = List.copyOf(readiness);
    this.children = List.copyOf(children);
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
    if (!children.isEmpty()) {
      d.set("children", ChildrenFfi.build(children));
    }
    return d;
  }

  public List<ReadinessChecksFfi.ReadinessEntry> readinessEntries() {
    return readiness;
  }
}
