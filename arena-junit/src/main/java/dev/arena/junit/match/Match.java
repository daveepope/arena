package dev.arena.junit.match;
import dev.arena.junit.exec.ContainerizedComponent;
import dev.arena.junit.exec.ExecutableComponent;
import dev.arena.junit.readiness.HttpReadinessCheck;
import dev.arena.junit.readiness.ReadinessChecksFfi;
import dev.arena.junit.readiness.ReadinessHooks;
import dev.arena.junit.support.ArenaJson;

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

  public List<ReadinessHooks.Hook> readinessHooks() {
    List<ReadinessHooks.Hook> out = new ArrayList<>();
    for (ArenaMatchPiece c : components) {
      List<ReadinessChecksFfi.ReadinessEntry> entries = List.of();
      if (c instanceof ExecutableComponent ec) {
        entries = ec.readinessEntries();
      } else if (c instanceof ContainerizedComponent cc) {
        entries = cc.readinessEntries();
      }
      String identifier = "";
      if (c instanceof ExecutableComponent ec) {
        identifier = ec.identifier();
      } else if (c instanceof ContainerizedComponent cc) {
        identifier = cc.identifier();
      }
      for (ReadinessChecksFfi.ReadinessEntry e : entries) {
        if (e.check() instanceof HttpReadinessCheck) {
          continue;
        }
        out.add(new ReadinessHooks.Hook(identifier, e.target(), e.check()));
      }
    }
    return out;
  }
}
