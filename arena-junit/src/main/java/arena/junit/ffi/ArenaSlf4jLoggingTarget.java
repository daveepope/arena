package arena.junit.ffi;

import com.sun.jna.Pointer;
import java.nio.charset.StandardCharsets;
import org.slf4j.Logger;

public final class ArenaSlf4jLoggingTarget implements ArenaLoggingTargetCallback {

  private final Logger logger;

  public ArenaSlf4jLoggingTarget(Logger logger) {
    if (logger == null) {
      throw new ArenaBindingError("slf4j logger is null");
    }
    this.logger = logger;
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
    String message =
        messageUtf8 != null
            ? messageUtf8.getString(0, StandardCharsets.UTF_8.name())
            : "";
    message =
        ArenaPlatformLoggingTarget.formatMessageWithRustCallerSuffix(
            message, callerFileUtf8, callerLine);
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
