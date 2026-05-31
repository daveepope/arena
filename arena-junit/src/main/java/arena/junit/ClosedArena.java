package arena.junit;

import arena.junit.ffi.ArenaBindingError;
import arena.junit.ffi.ArenaBindings;
import arena.junit.ffi.ArenaLogLevel;
import arena.junit.match.Match;
import arena.junit.support.ArenaJson;
import com.sun.jna.Pointer;
import java.util.List;
import org.slf4j.Logger;

public final class ClosedArena {
  private final String name;
  private final List<Match> matches;
  private final ArenaLogLevel logLevel;
  private final Logger slf4jLogger;
  private final List<String> logDependencyIds;
  private final List<String> logComponentIds;

  public ClosedArena(String name, List<Match> matches) {
    this(name, matches, ArenaLogLevel.INFO, null, List.of(), List.of());
  }

  public ClosedArena(String name, List<Match> matches, ArenaLogLevel logLevel) {
    this(name, matches, logLevel, null, List.of(), List.of());
  }

  public ClosedArena(String name, List<Match> matches, ArenaLogLevel logLevel, Logger slf4jLogger) {
    this(name, matches, logLevel, slf4jLogger, List.of(), List.of());
  }

  public ClosedArena(
      String name,
      List<Match> matches,
      ArenaLogLevel logLevel,
      Logger slf4jLogger,
      List<String> logComponentIds,
      List<String> logDependencyIds) {
    this.name = name;
    this.matches = List.copyOf(matches);
    this.logLevel = logLevel;
    this.slf4jLogger = slf4jLogger;
    this.logDependencyIds =
        logDependencyIds == null || logDependencyIds.isEmpty()
            ? List.of()
            : List.copyOf(logDependencyIds);
    this.logComponentIds =
        logComponentIds == null || logComponentIds.isEmpty()
            ? List.of()
            : List.copyOf(logComponentIds);
  }

  public OpenArena open() throws Exception {
    if (matches.isEmpty()) {
      throw new ArenaBindingError("closed arena has no matches");
    }
    String json = ArenaJson.MAPPER.writeValueAsString(matches.get(0).forFfi());
    ArenaBindings.setDispatcherDependencyAllowJson(
        logDependencyIds.isEmpty() ? null : ArenaJson.MAPPER.writeValueAsString(logDependencyIds));
    ArenaBindings.setDispatcherComponentAllowJson(
        logComponentIds.isEmpty() ? null : ArenaJson.MAPPER.writeValueAsString(logComponentIds));
    long logTok =
        slf4jLogger != null
            ? ArenaBindings.registerSlf4jDispatcherLoggingTarget(slf4jLogger, logLevel)
            : ArenaBindings.registerDefaultDispatcherLoggingTarget(logLevel);
    try {
      Pointer h = ArenaBindings.arenaOpen(name, json, logLevel);
      return new OpenArena(h, logTok, matches);
    } catch (ArenaBindingError e) {
      ArenaBindings.unregisterDispatcherLoggingTarget(logTok);
      throw e;
    }
  }
}
