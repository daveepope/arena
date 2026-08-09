package arena.junit.support;
import arena.junit.match.ArenaMatchPiece;

import com.fasterxml.jackson.databind.node.ArrayNode;
import java.util.List;

public final class ChildrenFfi {
  private ChildrenFfi() {}

  public static ArrayNode build(List<ArenaMatchPiece> children) {
    ArrayNode out = ArenaJson.array();
    for (ArenaMatchPiece child : children) {
      out.add(child.forFfi());
    }
    return out;
  }
}
