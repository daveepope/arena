package arena.junit.lifecycle;

import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

public final class LifecycleLog {
  public static final String ARENA_ROOT_LOGGER_NAME = "arena";

  private LifecycleLog() {}

  public static String arenaLoggerName(String arenaId) {
    String segment = arenaId == null ? "" : arenaId.trim().replace('.', '_');
    if (segment.isEmpty()) {
      return ARENA_ROOT_LOGGER_NAME;
    }
    return ARENA_ROOT_LOGGER_NAME + "." + segment;
  }

  public static void logTransition(ArenaState state) {
    Logger lg = LoggerFactory.getLogger(arenaLoggerName(state.id));
    int faultCount = state.faults.size();
    String line = faultCount > 0 ? state.state + " | faults=" + faultCount : state.state;
    if (state.isFaulted()) {
      lg.error(line);
    } else {
      lg.info(line);
    }
  }

  public static void logTransitionDocument(String document) {
    ArenaState state;
    try {
      state = ArenaState.parse(document);
    } catch (IllegalArgumentException e) {
      LoggerFactory.getLogger(ARENA_ROOT_LOGGER_NAME)
          .warn("unparseable arena state transition: {}", document);
      return;
    }
    logTransition(state);
  }

  public static void logClosingSummary(ArenaState state) {
    LoggerFactory.getLogger(arenaLoggerName(state.id))
        .info("closing summary | state={}, faults={}", state.state, state.faults.size());
  }

  public static void logClosingSummaryDocument(String document) {
    ArenaState state;
    try {
      state = ArenaState.parse(document);
    } catch (IllegalArgumentException e) {
      LoggerFactory.getLogger(ARENA_ROOT_LOGGER_NAME)
          .warn("unparseable arena closing state: {}", document);
      return;
    }
    logClosingSummary(state);
  }
}
