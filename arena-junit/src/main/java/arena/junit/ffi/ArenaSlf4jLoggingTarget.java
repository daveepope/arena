package arena.junit.ffi;

import com.sun.jna.Pointer;
import java.nio.charset.StandardCharsets;
import java.util.Map;
import java.util.concurrent.ConcurrentHashMap;
import org.slf4j.ILoggerFactory;
import org.slf4j.Logger;

public final class ArenaSlf4jLoggingTarget implements ArenaLoggingTargetCallback {

  static final String ROOT_LOGGER_NAME = "arena";

  private final Logger logger;
  private final ILoggerFactory loggerFactory;
  private final Map<String, Logger> loggersByName = new ConcurrentHashMap<>();

  public ArenaSlf4jLoggingTarget(Logger logger) {
    if (logger == null) {
      throw new ArenaBindingError("slf4j logger is null");
    }
    this.logger = logger;
    this.loggerFactory = null;
  }

  public ArenaSlf4jLoggingTarget(ILoggerFactory loggerFactory) {
    if (loggerFactory == null) {
      throw new ArenaBindingError("slf4j logger factory is null");
    }
    this.logger = null;
    this.loggerFactory = loggerFactory;
  }

  String loggerNameOf(Pointer targetUtf8) {
    if (targetUtf8 == null) {
      return "";
    }
    String name = targetUtf8.getString(0, StandardCharsets.UTF_8.name());
    return name == null ? "" : name;
  }

  String messageFor(String loggerName, String message) {
    if (loggerFactory != null || loggerName.isEmpty()) {
      return message;
    }
    return loggerName + "  " + message;
  }

  Logger loggerFor(String loggerName) {
    if (loggerFactory == null) {
      return logger;
    }
    String name = loggerName.isEmpty() ? ROOT_LOGGER_NAME : loggerName;
    return loggersByName.computeIfAbsent(name, loggerFactory::getLogger);
  }

  @Override
  @SuppressWarnings("unused")
  public void invoke(
      int level,
      Pointer targetUtf8,
      long ignoredTsNanos,
      Pointer messageUtf8,
      Pointer callerFileUtf8,
      int callerLine,
      Pointer ignoredUser) {
    int publish =
        ArenaBindings.lib().arena_dispatcher_default_logging_target_publish_level(level);
    String loggerName = loggerNameOf(targetUtf8);
    Logger logger = loggerFor(loggerName);
    String message =
        messageUtf8 != null
            ? messageUtf8.getString(0, StandardCharsets.UTF_8.name())
            : "";
    message =
        ArenaPlatformLoggingTarget.formatMessageWithRustCallerSuffix(
            message, callerFileUtf8, callerLine);
    message = messageFor(loggerName, message);
    if (publish == ArenaLogLevel.ERROR.code()) {
      logger.error(message);
      return;
    }
    if (publish == ArenaLogLevel.WARN.code()) {
      logger.warn(message);
      return;
    }
    if (publish == ArenaLogLevel.INFO.code()) {
      logger.info(message);
      return;
    }
    if (publish == ArenaLogLevel.DEBUG.code()) {
      logger.debug(message);
      return;
    }
    if (publish == ArenaLogLevel.TRACE.code()) {
      logger.trace(message);
      return;
    }
    logger.info(message);
  }
}
