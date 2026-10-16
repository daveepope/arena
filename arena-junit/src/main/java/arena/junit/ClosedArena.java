package arena.junit;

import arena.junit.ffi.ArenaBindingError;
import arena.junit.lifecycle.ArenaLifecycleError;
import arena.junit.ffi.ArenaBindings;
import arena.junit.ffi.ArenaLogLevel;
import arena.junit.match.Match;
import arena.junit.support.ArenaJson;
import com.sun.jna.Pointer;
import java.util.List;
import org.slf4j.ILoggerFactory;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

public final class ClosedArena {
  private static final Logger LOG = LoggerFactory.getLogger(ClosedArena.class);
  private static final int DEFAULT_DASHBOARD_PORT = 4873;

  private final String name;
  private final List<Match> matches;
  private final ArenaLogLevel logLevel;
  private final Logger slf4jLogger;
  private final ILoggerFactory slf4jLoggerFactory;
  private final List<String> logDependencyIds;
  private final List<String> logComponentIds;
  private final boolean lifecycleDashboard;
  private final String dashboardUrl;
  private final int dashboardPort;
  private final String dashboardSearchDir;

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
    this(
        name,
        matches,
        logLevel,
        slf4jLogger,
        slf4jLoggerFactory,
        logDependencyIds,
        logComponentIds,
        false,
        null,
        DEFAULT_DASHBOARD_PORT,
        null);
  }

  private ClosedArena(
      String name,
      List<Match> matches,
      ArenaLogLevel logLevel,
      Logger slf4jLogger,
      ILoggerFactory slf4jLoggerFactory,
      List<String> logDependencyIds,
      List<String> logComponentIds,
      boolean lifecycleDashboard,
      String dashboardUrl,
      int dashboardPort,
      String dashboardSearchDir) {
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
    this.lifecycleDashboard = lifecycleDashboard;
    this.dashboardUrl = dashboardUrl;
    this.dashboardPort = dashboardPort;
    this.dashboardSearchDir = dashboardSearchDir;
  }

  public ClosedArena withLifecycleDashboard() {
    return withLifecycleDashboardInternal(null, DEFAULT_DASHBOARD_PORT, null);
  }

  public ClosedArena withLifecycleDashboard(String url) {
    return withLifecycleDashboardInternal(url, DEFAULT_DASHBOARD_PORT, null);
  }

  public ClosedArena withLifecycleDashboard(int port, String searchDir) {
    return withLifecycleDashboardInternal(null, port, searchDir);
  }

  private ClosedArena withLifecycleDashboardInternal(String url, int port, String searchDir) {
    return new ClosedArena(
        name,
        matches,
        logLevel,
        slf4jLogger,
        slf4jLoggerFactory,
        logDependencyIds,
        logComponentIds,
        true,
        url,
        port,
        searchDir);
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

  private long registerDashboard() {
    if (!lifecycleDashboard) {
      return 0L;
    }
    try {
      String searchDir = dashboardSearchDir;
      if (searchDir == null || searchDir.isEmpty()) {
        String fromClasspath = ArenaBindings.dashboardSearchDirFromClasspath();
        searchDir = fromClasspath.isEmpty() ? null : fromClasspath;
      }
      String url =
          dashboardUrl != null
              ? dashboardUrl
              : ArenaBindings.dashboardEnsureRunning(dashboardPort, searchDir);
      return ArenaBindings.registerDashboardObserver(url);
    } catch (ArenaBindingError e) {
      LOG.warn("lifecycle dashboard unavailable, continuing without it", e);
      return 0L;
    }
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
    long dashboardTok = registerDashboard();
    try {
      Pointer h = ArenaBindings.arenaOpen(name, json, logLevel);
      return new OpenArena(h, logTok, dashboardTok, matches);
    } catch (ArenaBindingError e) {
      ArenaBindings.removeLifecycleObserver(dashboardTok);
      ArenaBindings.unregisterDispatcherLoggingTarget(logTok);
      throw ArenaLifecycleError.from(e);
    }
  }
}
