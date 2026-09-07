package arena.junit.ffi;

import static org.junit.jupiter.api.Assertions.*;

import ch.qos.logback.classic.Logger;
import ch.qos.logback.classic.LoggerContext;
import ch.qos.logback.classic.spi.ILoggingEvent;
import ch.qos.logback.core.read.ListAppender;
import com.sun.jna.Pointer;
import com.sun.jna.ptr.PointerByReference;
import java.io.ByteArrayOutputStream;
import java.io.PrintStream;
import java.nio.charset.StandardCharsets;
import java.util.Objects;
import java.util.stream.Stream;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.Assumptions;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.Arguments;
import org.junit.jupiter.params.provider.MethodSource;
import org.slf4j.LoggerFactory;

public final class DispatcherHostLoggingLifecycleTest {

  private static final String MATCH_JSON_ARENA_OPEN_BUILD_FAIL_WITH_NO_RUNTIME =
      "{\"dependencies\":[{\"type\":\"kafka\",\"identifier\":\"dispatcher-logging-junit-host\",\"flavor\":\"not_supported_for_dispatcher_host_logging_test\"}]}";

  private static final String RESTRICTIVE_DISPATCHER_ALLOW_JSON =
      "[\"nope-artificial-needle-not-in-product\"]";

  @AfterEach
  void arenaDispatcherAllowsClearedAfterEachTest() {
    if (ArenaNativeHolder.LIB == null) {
      return;
    }
    ArenaBindings.setDispatcherDependencyAllowJson(null);
    ArenaBindings.setDispatcherComponentAllowJson(null);
  }

  static Stream<Arguments> registerSlf4jArenaSetLogLevelFfiLevelCarrierMatchesFloorSource() {
    return Stream.of(
        Arguments.of(ArenaLogLevel.ERROR, "Error", false),
        Arguments.of(ArenaLogLevel.WARN, "Warn", false),
        Arguments.of(ArenaLogLevel.INFO, "Info", true),
        Arguments.of(ArenaLogLevel.DEBUG, "Debug", true),
        Arguments.of(ArenaLogLevel.TRACE, "Trace", true));
  }

  @ParameterizedTest
  @MethodSource("registerSlf4jArenaSetLogLevelFfiLevelCarrierMatchesFloorSource")
  void registerSlf4jArenaSetLogLevelFfiLevelCarrierMatchesFloor(
      ArenaLogLevel ffiLevel, String rustLevelToken, boolean expectInfoCarrier) {
    Assumptions.assumeTrue(ArenaNativeHolder.LIB != null);

    LoggerContext cx = backlogContext();
    String nm =
        "arena.junit.dispatcher.host." + Objects.hash(cx, Thread.currentThread(), System.nanoTime());
    Logger slog = backlogLogger(nm);
    ListAppender<ILoggingEvent> cap = new ListAppender<>();
    cap.setContext(cx);
    cap.setName("capture");
    cap.start();
    slog.addAppender(cap);

    long tok = ArenaBindings.registerSlf4jDispatcherLoggingTarget(slog, ArenaLogLevel.TRACE);
    try {
      cap.list.clear();
      ArenaBindings.lib().arena_set_log_level(ffiLevel.code());
      ArenaLogbackFlush.flushIfPresent();
      if (!expectInfoCarrier) {
        assertTrue(
            cap.list.stream().noneMatch(ev -> safeMessage(ev).contains("arena log level set")),
            cap.list::toString);
        return;
      }
      long infoMarkers =
          cap.list.stream()
              .filter(ev -> ev.getLevel() != null && "INFO".equals(ev.getLevel().toString()))
              .filter(ev -> safeMessage(ev).contains("arena log level set"))
              .count();
      assertEquals(1L, infoMarkers);
      assertTrue(
          cap.list.stream()
              .anyMatch(ev -> safeMessage(ev).contains("arena_log_level=" + rustLevelToken)),
          cap.list::toString);
    } finally {
      ArenaBindings.unregisterDispatcherLoggingTarget(tok);
      cap.stop();
      slog.detachAppender(cap);
    }
  }

  @ParameterizedTest
  @MethodSource("registerSlf4jArenaSetLogLevelFfiLevelCarrierMatchesFloorSource")
  void registerDefaultDispatcherArenaSetLogLevelFfiLevelStderrMatchesFloor(
      ArenaLogLevel ffiLevel, String rustLevelToken, boolean expectInfoCarrier) {
    Assumptions.assumeTrue(ArenaNativeHolder.LIB != null);

    PrintStream priorErr = System.err;
    ByteArrayOutputStream buf = new ByteArrayOutputStream();
    PrintStream captureErr = new PrintStream(buf, true, StandardCharsets.UTF_8);
    System.setErr(captureErr);
    try {
      long tok = ArenaBindings.registerDefaultDispatcherLoggingTarget(ArenaLogLevel.TRACE);
      try {
        ArenaBindings.lib().arena_set_log_level(ffiLevel.code());
        ArenaLogbackFlush.flushIfPresent();
      } finally {
        ArenaBindings.unregisterDispatcherLoggingTarget(tok);
      }
      captureErr.flush();
      String errText = buf.toString(StandardCharsets.UTF_8);
      if (!expectInfoCarrier) {
        assertFalse(errText.contains("arena log level set"), () -> errText);
        return;
      }
      assertTrue(errText.contains("arena log level set"), () -> errText);
      assertTrue(errText.contains("arena_log_level=" + rustLevelToken), () -> errText);
    } finally {
      System.setErr(priorErr);
    }
  }

  @Test
  void registerSlf4jArenaOpenBuildFailInsideLibForwardsOpenFailedMarkerToRealLogger() {
    Assumptions.assumeTrue(ArenaNativeHolder.LIB != null);

    LoggerContext cx = backlogContext();
    String nm =
        "arena.junit.dispatcher.openfail."
            + Objects.hash(cx, Thread.currentThread(), System.nanoTime());
    Logger slog = backlogLogger(nm);
    ListAppender<ILoggingEvent> cap = new ListAppender<>();
    cap.setContext(cx);
    cap.setName("capture");
    cap.start();
    slog.addAppender(cap);

    long tok = ArenaBindings.registerSlf4jDispatcherLoggingTarget(slog, ArenaLogLevel.TRACE);
    try {
      ArenaBindings.lib().arena_set_log_level(ArenaLogLevel.TRACE.code());
      cap.list.clear();

      PointerByReference err = new PointerByReference();
      PointerByReference state = new PointerByReference();
      Pointer h =
          ArenaBindings.lib()
              .arena_open(
                  "junit-dispatcher-host-logging-binding",
                  MATCH_JSON_ARENA_OPEN_BUILD_FAIL_WITH_NO_RUNTIME, err, state);
      ArenaBindings.takeOutString(state);
      assertTrue(h == null || Pointer.nativeValue(h) == 0L);
      assertNotNull(ArenaBindings.takeErr(err));

      ArenaLogbackFlush.flushIfPresent();
      assertTrue(
          cap.list.stream()
                  .filter(ev -> ev.getLevel() != null && "ERROR".equals(ev.getLevel().toString()))
                  .count()
              >= 1L,
          () -> cap.list.toString());
      assertTrue(
          cap.list.stream()
              .filter(ev -> ev.getLevel() != null && "ERROR".equals(ev.getLevel().toString()))
              .anyMatch(
                  ev ->
                      safeMessage(ev).contains("open failed")
                          || safeMessage(ev).contains("arena_open")),
          () -> cap.list.toString());

    } finally {
      ArenaBindings.unregisterDispatcherLoggingTarget(tok);
      cap.stop();
      slog.detachAppender(cap);
    }
  }

  @Test
  void registerSlf4jCustomLoggerThenUnregisterRestoresStderrAndAdditive() {
    Assumptions.assumeTrue(ArenaNativeHolder.LIB != null);

    LoggerContext cx = backlogContext();
    String nm =
        "arena.junit.dispatcher.unregister."
            + Objects.hash(cx, Thread.currentThread(), System.nanoTime());
    Logger slog = backlogLogger(nm);
    assertTrue(slog.isAdditive(), slog::toString);

    long tok = ArenaBindings.registerSlf4jDispatcherLoggingTarget(slog, ArenaLogLevel.INFO);
    try {
      assertFalse(slog.isAdditive(), slog::toString);
      assertNotNull(
          slog.getAppender(ArenaSlf4jDispatcherStderrPublication.STDERR_APPENDER_NAME),
          slog::toString);
    } finally {
      ArenaBindings.unregisterDispatcherLoggingTarget(tok);
    }

    assertTrue(slog.isAdditive(), slog::toString);
    assertNull(
        slog.getAppender(ArenaSlf4jDispatcherStderrPublication.STDERR_APPENDER_NAME), slog::toString);
  }

  @Test
  void registerSlf4jCustomLoggerRestrictiveDispatcherAllowsStillForwardsArenaFfiSetLevel() {
    Assumptions.assumeTrue(ArenaNativeHolder.LIB != null);
    ArenaBindings.setDispatcherDependencyAllowJson(RESTRICTIVE_DISPATCHER_ALLOW_JSON);
    ArenaBindings.setDispatcherComponentAllowJson(RESTRICTIVE_DISPATCHER_ALLOW_JSON);

    LoggerContext cx = backlogContext();
    String nm =
        "arena.junit.dispatcher.allowgate."
            + Objects.hash(cx, Thread.currentThread(), System.nanoTime());
    Logger slog = backlogLogger(nm);
    ListAppender<ILoggingEvent> cap = new ListAppender<>();
    cap.setContext(cx);
    cap.setName("capture");
    cap.start();
    slog.addAppender(cap);

    long tok = ArenaBindings.registerSlf4jDispatcherLoggingTarget(slog, ArenaLogLevel.TRACE);
    try {
      cap.list.clear();
      ArenaBindings.lib().arena_set_log_level(ArenaLogLevel.INFO.code());
      ArenaLogbackFlush.flushIfPresent();
      assertTrue(
          cap.list.stream().anyMatch(ev -> safeMessage(ev).contains("arena log level set")),
          cap.list::toString);
    } finally {
      ArenaBindings.unregisterDispatcherLoggingTarget(tok);
      cap.stop();
      slog.detachAppender(cap);
    }
  }

  @Test
  void registerSlf4jCustomLoggerRestrictiveDispatcherAllowsStillForwardsArenaOpenError() {
    Assumptions.assumeTrue(ArenaNativeHolder.LIB != null);
    ArenaBindings.setDispatcherDependencyAllowJson(RESTRICTIVE_DISPATCHER_ALLOW_JSON);
    ArenaBindings.setDispatcherComponentAllowJson(RESTRICTIVE_DISPATCHER_ALLOW_JSON);

    LoggerContext cx = backlogContext();
    String nm =
        "arena.junit.dispatcher.allowopen."
            + Objects.hash(cx, Thread.currentThread(), System.nanoTime());
    Logger slog = backlogLogger(nm);
    ListAppender<ILoggingEvent> cap = new ListAppender<>();
    cap.setContext(cx);
    cap.setName("capture");
    cap.start();
    slog.addAppender(cap);

    long tok = ArenaBindings.registerSlf4jDispatcherLoggingTarget(slog, ArenaLogLevel.TRACE);
    try {
      ArenaBindings.lib().arena_set_log_level(ArenaLogLevel.TRACE.code());
      cap.list.clear();

      PointerByReference err = new PointerByReference();
      PointerByReference state = new PointerByReference();
      Pointer h =
          ArenaBindings.lib()
              .arena_open(
                  "junit-dispatcher-host-logging-binding-open-error",
                  MATCH_JSON_ARENA_OPEN_BUILD_FAIL_WITH_NO_RUNTIME, err, state);
      ArenaBindings.takeOutString(state);
      assertTrue(h == null || Pointer.nativeValue(h) == 0L);
      assertNotNull(ArenaBindings.takeErr(err));

      ArenaLogbackFlush.flushIfPresent();
      assertTrue(
          cap.list.stream()
                  .filter(ev -> ev.getLevel() != null && "ERROR".equals(ev.getLevel().toString()))
                  .count()
              >= 1L,
          cap.list::toString);
      assertTrue(
          cap.list.stream()
              .filter(ev -> ev.getLevel() != null && "ERROR".equals(ev.getLevel().toString()))
              .anyMatch(
                  ev ->
                      safeMessage(ev).contains("open failed")
                          || safeMessage(ev).contains("arena_open")),
          cap.list::toString);
    } finally {
      ArenaBindings.unregisterDispatcherLoggingTarget(tok);
      cap.stop();
      slog.detachAppender(cap);
    }
  }

  private static String safeMessage(ILoggingEvent ev) {
    String formatted = ev.getFormattedMessage();
    if (formatted != null && !formatted.isEmpty()) {
      return formatted;
    }
    String raw = ev.getMessage();
    return raw != null ? raw : "";
  }

  private static LoggerContext backlogContext() {
    if (!(LoggerFactory.getILoggerFactory() instanceof LoggerContext)) {
      throw new AssertionError("Logback LoggerContext expected on test classpath");
    }
    return (LoggerContext) LoggerFactory.getILoggerFactory();
  }

  private static Logger backlogLogger(String name) {
    org.slf4j.Logger facade = LoggerFactory.getLogger(name);
    if (!(facade instanceof Logger)) {
      throw new AssertionError("org.slf4j.Logger must bridge to Logback classic Logger here");
    }
    return (Logger) facade;
  }
}
