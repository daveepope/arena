package arena.junit;

import arena.junit.ffi.ArenaBindingError;
import arena.junit.ffi.ArenaBindings;
import arena.junit.ffi.ArenaLogLevel;
import arena.junit.match.Match;
import arena.junit.support.ArenaJson;
import com.sun.jna.Pointer;
import java.util.List;
import org.slf4j.ILoggerFactory;
import org.slf4j.Logger;

public final class ClosedArena {
  private final String name;
  private final List<Match> matches;
  private final ArenaLogLevel logLevel;
  private final Logger slf4jLogger;
  private final ILoggerFactory slf4jLoggerFactory;
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
      ILoggerFactory slf4jLoggerFactory) {
    this(name, matches, logLevel, null, slf4jLoggerFactory, List.of(), List.of());
  }

  public ClosedArena(
      String name,
      List<Match> matches,
      ArenaLogLevel logLevel,
      Logger slf4jLogger,
      List<String> logDependencyIds,
      List<String> logComponentIds) {
    this(name, matches, logLevel, slf4jLogger, null, logDependencyIds, logComponentIds);
  }

  public ClosedArena(
      String name,
      List<Match> matches,
      ArenaLogLevel logLevel,
      Logger slf4jLogger,
      ILoggerFactory slf4jLoggerFactory,
      List<String> logDependencyIds,
      List<String> logComponentIds) {
    this.name = name;
    this.matches = List.copyOf(matches);
    this.logLevel = logLevel;
    this.slf4jLogger = slf4jLogger;
    this.slf4jLoggerFactory = slf4jLoggerFactory;
    this.logDependencyIds =
        logDependencyIds == null || logDependencyIds.isEmpty()
            ? List.of()
            : List.copyOf(logDependencyIds);
    this.logComponentIds =
        logComponentIds == null || logComponentIds.isEmpty()
            ? List.of()
            : List.copyOf(logComponentIds);
  }

  private long registerLoggingTarget() {
    if (slf4jLoggerFactory != null) {
      return ArenaBindings.registerSlf4jDispatcherLoggingTarget(slf4jLoggerFactory, logLevel);
    }
    if (slf4jLogger != null) {
      return ArenaBindings.registerSlf4jDispatcherLoggingTarget(slf4jLogger, logLevel);
    }
    return ArenaBindings.registerDefaultDispatcherLoggingTarget(logLevel);
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
    long logTok = registerLoggingTarget();
    try {
      Pointer h = ArenaBindings.arenaOpen(name, json, logLevel);
      return new OpenArena(h, logTok, matches);
    } catch (ArenaBindingError e) {
      ArenaBindings.unregisterDispatcherLoggingTarget(logTok);
      throw e;
    }
  }
}
