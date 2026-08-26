package arena.junit.support;
import arena.junit.match.ArenaRunnableComponent;
import arena.junit.match.ArenaRunnableDependency;

import com.fasterxml.jackson.databind.node.ArrayNode;
import java.util.List;

public final class ChildrenFfi {
  private ChildrenFfi() {}

  public static ArrayNode buildComponents(List<ArenaRunnableComponent> children) {
    ArrayNode out = ArenaJson.array();
    for (ArenaRunnableComponent child : children) {
      out.add(child.forFfi());
    }
    return out;
  }

  public static ArrayNode buildDependencies(List<ArenaRunnableDependency> children) {
    ArrayNode out = ArenaJson.array();
    for (ArenaRunnableDependency child : children) {
      out.add(child.forFfi());
    }
    return out;
  }
}
