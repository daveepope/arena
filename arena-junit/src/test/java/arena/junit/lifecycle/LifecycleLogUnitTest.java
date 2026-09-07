package arena.junit.lifecycle;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;

import ch.qos.logback.classic.Level;
import ch.qos.logback.classic.Logger;
import ch.qos.logback.classic.spi.ILoggingEvent;
import ch.qos.logback.core.read.ListAppender;
import java.util.List;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.CsvSource;
import org.slf4j.LoggerFactory;

class LifecycleLogUnitTest {

  private static ListAppender<ILoggingEvent> attach(String loggerName) {
    Logger lg = (Logger) LoggerFactory.getLogger(loggerName);
    ListAppender<ILoggingEvent> capture = new ListAppender<>();
    capture.start();
    lg.addAppender(capture);
    lg.setLevel(Level.DEBUG);
    return capture;
  }

  private static void detach(String loggerName, ListAppender<ILoggingEvent> capture) {
    Logger lg = (Logger) LoggerFactory.getLogger(loggerName);
    lg.detachAppender(capture);
    lg.setLevel(null);
  }

  @ParameterizedTest
  @CsvSource({
    "orders, arena.orders",
    "orders.v2, arena.orders_v2",
    "'  ', arena",
    "'', arena"
  })
  void arenaLoggerNameIdentifierMatchesDispatcherNamespace(String arenaId, String expected) {
    assertEquals(expected, LifecycleLog.arenaLoggerName(arenaId));
  }

  @Test
  void logTransitionCleanStateLogsInfoUnderArenaLogger() {
    ArenaState state =
        ArenaState.parse(
            "{\"id\":\"transition-clean\",\"state\":\"dependencies_starting\",\"at\":\"t\"}");
    ListAppender<ILoggingEvent> capture = attach("arena.transition-clean");
    try {
      LifecycleLog.logTransition(state);
    } finally {
      detach("arena.transition-clean", capture);
    }

    List<ILoggingEvent> events = capture.list;
    assertEquals(1, events.size());
    assertEquals(Level.INFO, events.get(0).getLevel());
    assertEquals("dependencies_starting", events.get(0).getFormattedMessage());
  }

  @Test
  void logTransitionFaultedStateLogsErrorWithFaultCount() {
    ArenaState state = ArenaState.parse(ArenaStateUnitTest.FIXTURE_STATE_JSON);
    ListAppender<ILoggingEvent> capture = attach("arena.orders");
    try {
      LifecycleLog.logTransition(state);
    } finally {
      detach("arena.orders", capture);
    }

    assertEquals(1, capture.list.size());
    assertEquals(Level.ERROR, capture.list.get(0).getLevel());
    assertEquals("arena_faulted | faults=1", capture.list.get(0).getFormattedMessage());
  }

  @Test
  void logClosingSummaryStateLogsTokenAndFaultCount() {
    ArenaState state =
        ArenaState.parse("{\"id\":\"closing-summary\",\"state\":\"arena_closed\",\"at\":\"t\"}");
    ListAppender<ILoggingEvent> capture = attach("arena.closing-summary");
    try {
      LifecycleLog.logClosingSummary(state);
    } finally {
      detach("arena.closing-summary", capture);
    }

    assertEquals(
        "closing summary | state=arena_closed, faults=0",
        capture.list.get(0).getFormattedMessage());
  }

  @Test
  void logTransitionDocumentValidDocumentLogsTheTransition() {
    ListAppender<ILoggingEvent> capture = attach("arena.doc-valid");
    try {
      LifecycleLog.logTransitionDocument(
          "{\"id\":\"doc-valid\",\"state\":\"arena_open\",\"at\":\"t\"}");
    } finally {
      detach("arena.doc-valid", capture);
    }

    assertEquals("arena_open", capture.list.get(0).getFormattedMessage());
  }

  @Test
  void logClosingSummaryDocumentValidDocumentLogsTheSummary() {
    ListAppender<ILoggingEvent> capture = attach("arena.close-doc-valid");
    try {
      LifecycleLog.logClosingSummaryDocument(
          "{\"id\":\"close-doc-valid\",\"state\":\"arena_closed\",\"at\":\"t\"}");
    } finally {
      detach("arena.close-doc-valid", capture);
    }

    assertEquals(
        "closing summary | state=arena_closed, faults=0",
        capture.list.get(0).getFormattedMessage());
  }

  @Test
  void logTransitionDocumentUnparseableDocumentWarnsOnRootLogger() {
    ListAppender<ILoggingEvent> capture = attach("arena");
    try {
      LifecycleLog.logTransitionDocument("{broken");
    } finally {
      detach("arena", capture);
    }

    assertFalse(capture.list.isEmpty());
    assertEquals(Level.WARN, capture.list.get(0).getLevel());
  }

  @Test
  void logClosingSummaryDocumentUnparseableDocumentWarnsOnRootLogger() {
    ListAppender<ILoggingEvent> capture = attach("arena");
    try {
      LifecycleLog.logClosingSummaryDocument("{broken");
    } finally {
      detach("arena", capture);
    }

    assertFalse(capture.list.isEmpty());
    assertEquals(Level.WARN, capture.list.get(0).getLevel());
  }
}
