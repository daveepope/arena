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
  private final List<String> cpuProfiledComponents;
  private final Long cpuProfileLimitBytes;

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
        null,
        List.of(),
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
      String dashboardSearchDir,
      List<String> cpuProfiledComponents,
      Long cpuProfileLimitBytes) {
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
    this.cpuProfiledComponents = List.copyOf(cpuProfiledComponents);
    this.cpuProfileLimitBytes = cpuProfileLimitBytes;
  }

  public LifecycleDashboardBuilder withLifecycleDashboard() {
    return new LifecycleDashboardBuilder(
        withLifecycleDashboardInternal(null, DEFAULT_DASHBOARD_PORT, null));
  }

  public LifecycleDashboardBuilder withLifecycleDashboard(String url) {
    return new LifecycleDashboardBuilder(
        withLifecycleDashboardInternal(url, DEFAULT_DASHBOARD_PORT, null));
  }

  public LifecycleDashboardBuilder withLifecycleDashboard(int port, String searchDir) {
    return new LifecycleDashboardBuilder(withLifecycleDashboardInternal(null, port, searchDir));
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
        searchDir,
        cpuProfiledComponents,
        cpuProfileLimitBytes);
  }

  ClosedArena withCpuProfiledComponent(String componentId) {
    List<String> next = new java.util.ArrayList<>(cpuProfiledComponents);
    next.add(componentId);
    return new ClosedArena(
        name,
        matches,
        logLevel,
        slf4jLogger,
        slf4jLoggerFactory,
        logDependencyIds,
        logComponentIds,
        lifecycleDashboard,
        dashboardUrl,
        dashboardPort,
        dashboardSearchDir,
        next,
        cpuProfileLimitBytes);
  }

  ClosedArena withCpuProfileLimitBytes(long limitBytes) {
    return new ClosedArena(
        name,
        matches,
        logLevel,
        slf4jLogger,
        slf4jLoggerFactory,
        logDependencyIds,
        logComponentIds,
        lifecycleDashboard,
        dashboardUrl,
        dashboardPort,
        dashboardSearchDir,
        cpuProfiledComponents,
        limitBytes);
  }

  com.fasterxml.jackson.databind.node.ObjectNode matchConfigForFfi() {
    com.fasterxml.jackson.databind.node.ObjectNode config = matches.get(0).forFfi().deepCopy();
    if (!cpuProfiledComponents.isEmpty()) {
      com.fasterxml.jackson.databind.node.ArrayNode ids =
          config.putArray("cpu_profiled_components");
      cpuProfiledComponents.forEach(ids::add);
    }
    if (cpuProfileLimitBytes != null) {
      config.put("cpu_profile_limit_bytes", cpuProfileLimitBytes.longValue());
    }
    return config;
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
    String json = ArenaJson.MAPPER.writeValueAsString(matchConfigForFfi());
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
