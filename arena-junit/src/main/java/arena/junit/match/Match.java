package arena.junit.match;

import arena.junit.support.ArenaJson;

import com.fasterxml.jackson.databind.node.ArrayNode;
import com.fasterxml.jackson.databind.node.ObjectNode;
import java.util.ArrayList;
import java.util.List;

public final class Match {
  private final String name;
  private final List<ArenaMatchPiece> dependencies = new ArrayList<>();
  private final List<ArenaMatchPiece> components = new ArrayList<>();
  private final List<RegisteredPlaybook> playbooks = new ArrayList<>();
  private String network;

  Match(
      String name,
      List<ArenaMatchPiece> dependencies,
      List<ArenaMatchPiece> components,
      String network,
      List<RegisteredPlaybook> playbooks) {
    this.name = name;
    this.dependencies.addAll(dependencies);
    this.components.addAll(components);
    this.network = network;
    this.playbooks.addAll(playbooks);
  }

  public ObjectNode forFfi() {
    ObjectNode out = ArenaJson.object();
    out.put("match_name", name);
    ArrayNode deps = ArenaJson.array();
    for (ArenaMatchPiece d : dependencies) {
      deps.add(d.forFfi());
    }
    out.set("dependencies", deps);
    ArrayNode comps = ArenaJson.array();
    for (ArenaMatchPiece c : components) {
      comps.add(c.forFfi());
    }
    out.set("components", comps);
    if (network != null && !network.isEmpty()) {
      out.put("network", network);
    }
    if (!playbooks.isEmpty()) {
      ArrayNode pbs = ArenaJson.array();
      for (RegisteredPlaybook p : playbooks) {
        pbs.add(p.forFfi());
      }
      out.set("playbooks", pbs);
    }
    return out;
  }
}
