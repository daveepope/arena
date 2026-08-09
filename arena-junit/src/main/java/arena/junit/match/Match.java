package arena.junit.match;

import arena.junit.playbook.Playbook;
import arena.junit.support.ArenaJson;
import arena.junit.support.ChildrenFfi;

import com.fasterxml.jackson.databind.node.ArrayNode;
import com.fasterxml.jackson.databind.node.ObjectNode;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

public final class Match {
  private final String name;
  private final List<ArenaMatchPiece> dependencies = new ArrayList<>();
  private final List<ArenaMatchPiece> components = new ArrayList<>();
  private final Map<Class<? extends Playbook>, RegisteredPlaybook> playbooks =
      new LinkedHashMap<>();
  private String network;

  Match(
      String name,
      List<ArenaMatchPiece> dependencies,
      List<ArenaMatchPiece> components,
      String network,
      Map<Class<? extends Playbook>, RegisteredPlaybook> playbooks) {
    this.name = name;
    this.dependencies.addAll(dependencies);
    this.components.addAll(components);
    this.network = network;
    this.playbooks.putAll(playbooks);
  }

  public String name() {
    return name;
  }

  public Playbook playbook(Class<? extends Playbook> klass) {
    RegisteredPlaybook rp = playbooks.get(klass);
    return rp == null ? null : rp.playbook();
  }

  public Boolean execOnDependencyStart(Class<? extends Playbook> klass) {
    RegisteredPlaybook rp = playbooks.get(klass);
    return rp == null ? null : rp.execOnDependencyStart();
  }

  public ObjectNode forFfi() {
    ObjectNode out = ArenaJson.object();
    out.put("match_name", name);
    out.set("dependencies", ChildrenFfi.build(dependencies));
    out.set("components", ChildrenFfi.build(components));
    if (network != null && !network.isEmpty()) {
      out.put("network", network);
    }
    ArrayNode pbs = ArenaJson.array();
    for (RegisteredPlaybook p : playbooks.values()) {
      ObjectNode serialized = p.forFfi();
      if (serialized != null) {
        pbs.add(serialized);
      }
    }
    if (!pbs.isEmpty()) {
      out.set("playbooks", pbs);
    }
    return out;
  }
}
