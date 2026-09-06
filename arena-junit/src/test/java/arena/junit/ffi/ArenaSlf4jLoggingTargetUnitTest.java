package arena.junit.ffi;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertSame;
import static org.junit.jupiter.api.Assertions.assertThrows;

import java.util.HashMap;
import java.util.Map;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.CsvSource;
import org.slf4j.ILoggerFactory;
import org.slf4j.Logger;
import org.slf4j.helpers.NOPLogger;

class ArenaSlf4jLoggingTargetUnitTest {

  private static final class RecordingLoggerFactory implements ILoggerFactory {
    final Map<String, Logger> requested = new HashMap<>();

    @Override
    public Logger getLogger(String name) {
      return requested.computeIfAbsent(name, ignored -> NOPLogger.NOP_LOGGER);
    }
  }

  @ParameterizedTest
  @CsvSource({
    "arena.orders, arena.orders",
    "arena.orders.dependency.orders-postgres, arena.orders.dependency.orders-postgres",
    "'', arena"
  })
  void loggerForRecordTargetResolvesThroughTheFactory(String target, String expectedName) {
    RecordingLoggerFactory loggerFactory = new RecordingLoggerFactory();
    ArenaSlf4jLoggingTarget loggingTarget = new ArenaSlf4jLoggingTarget(loggerFactory);

    loggingTarget.loggerFor(target);

    assertEquals(Map.of(expectedName, NOPLogger.NOP_LOGGER), loggerFactory.requested);
  }

  @Test
  void loggerForRepeatedTargetReusesTheCachedLogger() {
    RecordingLoggerFactory loggerFactory = new RecordingLoggerFactory();
    ArenaSlf4jLoggingTarget loggingTarget = new ArenaSlf4jLoggingTarget(loggerFactory);

    Logger first = loggingTarget.loggerFor("arena.orders");
    Logger second = loggingTarget.loggerFor("arena.orders");

    assertSame(first, second);
    assertEquals(1, loggerFactory.requested.size());
  }

  @Test
  void loggerForBareLoggerReturnsThatLogger() {
    ArenaSlf4jLoggingTarget loggingTarget = new ArenaSlf4jLoggingTarget(NOPLogger.NOP_LOGGER);

    assertSame(NOPLogger.NOP_LOGGER, loggingTarget.loggerFor("arena.orders"));
  }

  @Test
  void messageForBareLoggerPrefixesTheLoggerName() {
    ArenaSlf4jLoggingTarget loggingTarget = new ArenaSlf4jLoggingTarget(NOPLogger.NOP_LOGGER);

    assertEquals("arena.orders  started", loggingTarget.messageFor("arena.orders", "started"));
  }

  @Test
  void messageForLoggerFactoryLeavesTheMessageUnchanged() {
    ArenaSlf4jLoggingTarget loggingTarget =
        new ArenaSlf4jLoggingTarget(new RecordingLoggerFactory());

    assertEquals("started", loggingTarget.messageFor("arena.orders", "started"));
  }

  @Test
  void messageForBlankLoggerNameLeavesTheMessageUnchanged() {
    ArenaSlf4jLoggingTarget loggingTarget = new ArenaSlf4jLoggingTarget(NOPLogger.NOP_LOGGER);

    assertEquals("started", loggingTarget.messageFor("", "started"));
  }

  @Test
  void constructorNullLoggerFactoryRaisesBindingError() {
    assertThrows(
        ArenaBindingError.class, () -> new ArenaSlf4jLoggingTarget((ILoggerFactory) null));
  }
}
