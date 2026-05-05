package dev.arena.junit;

import com.fasterxml.jackson.databind.node.ObjectNode;
import com.sun.jna.Pointer;
import dev.arena.junit.ffi.ArenaBindingError;
import dev.arena.junit.ffi.ArenaBindings;
import dev.arena.junit.match.Match;
import dev.arena.junit.readiness.HttpReadinessCheck;
import dev.arena.junit.readiness.ReadinessDefaults;
import dev.arena.junit.readiness.ReadinessHooks;
import dev.arena.junit.readiness.RunReadiness;
import dev.arena.junit.support.ArenaJson;
import java.util.List;

public final class ClosedArena {
  private final String name;
  private final List<Match> matches;

  public ClosedArena(String name, List<Match> matches) {
    this.name = name;
    this.matches = List.copyOf(matches);
  }

  public OpenArena open() throws Exception {
    if (matches.isEmpty()) {
      throw new ArenaBindingError("closed arena has no matches");
    }
    String json = ArenaJson.MAPPER.writeValueAsString(matches.get(0).forFfi());
    Pointer h = ArenaBindings.arenaOpen(name, json);
    OpenArena arena = new OpenArena(h);
    for (ReadinessHooks.Hook hook : matches.get(0).readinessHooks()) {
      if (!(hook.check() instanceof HttpReadinessCheck)) {
        RunReadiness.runReadiness(
            hook.check(), hook.identifier(), hook.target(), ReadinessDefaults.DEFAULT_READINESS_TIMEOUT_MS);
      }
    }
    return arena;
  }
}
