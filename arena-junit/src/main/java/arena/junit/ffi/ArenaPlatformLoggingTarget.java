package arena.junit.ffi;

import com.sun.jna.Pointer;
import java.util.logging.ConsoleHandler;
import java.util.logging.Handler;
import java.util.logging.Level;
import java.util.logging.LogRecord;
import java.util.logging.Logger;

final class ArenaPlatformLoggingTarget implements ArenaLoggingTargetCallback {

  static final ArenaPlatformLoggingTarget INSTANCE = new ArenaPlatformLoggingTarget();

  private static final class ArenaJulDispatcherConsoleHandler extends ConsoleHandler {}

  private static volatile Logger dispatcherJvmLogger;
  private static Boolean julSavedUseParentHandlers;

  private ArenaPlatformLoggingTarget() {}

  static String formatMessageWithRustCallerSuffix(
      String message, Pointer callerFileUtf8, int callerLine) {
    if (callerFileUtf8 == null || Pointer.nativeValue(callerFileUtf8) == 0 || callerLine <= 0) {
      return message;
    }
    String fileOnly = callerFileUtf8.getString(0, java.nio.charset.StandardCharsets.UTF_8.name());
    return message + " [" + fileOnly + ":" + callerLine + "]";
  }

  static void installJulDirectStderr(ArenaLogLevel arenaLogLevel) {
    synchronized (ArenaPlatformLoggingTarget.class) {
      Logger lg = ensureJulDispatcherLoggerLocked();
      lg.setLevel(julFloorForArena(arenaLogLevel));
      for (Handler h : lg.getHandlers()) {
        if (h instanceof ArenaJulDispatcherConsoleHandler) {
          return;
        }
      }
      julSavedUseParentHandlers = lg.getUseParentHandlers();
      lg.setUseParentHandlers(false);
      ArenaJulDispatcherConsoleHandler ch = new ArenaJulDispatcherConsoleHandler();
      ch.setLevel(Level.ALL);
      lg.addHandler(ch);
    }
  }

  static void removeJulDirectStderrInstallation() {
    synchronized (ArenaPlatformLoggingTarget.class) {
      Logger lg = dispatcherJvmLogger;
      if (lg == null) {
        return;
      }
      for (Handler h : lg.getHandlers()) {
        if (h instanceof ArenaJulDispatcherConsoleHandler) {
          lg.removeHandler(h);
          h.close();
        }
      }
      if (julSavedUseParentHandlers != null) {
        lg.setUseParentHandlers(julSavedUseParentHandlers.booleanValue());
        julSavedUseParentHandlers = null;
      }
    }
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
    Logger jul = jvmJulLoggerForDispatcherDefaultLoggingTarget();
    int publishCode =
        ArenaBindings.lib().arena_dispatcher_default_logging_target_publish_level(level);
    Level julPublish = julPublishLevelForDispatcherLoggingTarget(publishCode);
    String message =
        messageUtf8 != null
            ? messageUtf8.getString(0, java.nio.charset.StandardCharsets.UTF_8.name())
            : "";
    message = formatMessageWithRustCallerSuffix(message, callerFileUtf8, callerLine);
    LogRecord record = new LogRecord(julPublish, message);
    record.setLoggerName(jul.getName());
    record.setSourceClassName(jul.getName());
    record.setSourceMethodName("publish");
    jul.log(record);
  }

  private static Logger jvmJulLoggerForDispatcherDefaultLoggingTarget() {
    Logger cached = dispatcherJvmLogger;
    if (cached != null) {
      return cached;
    }
    synchronized (ArenaPlatformLoggingTarget.class) {
      return ensureJulDispatcherLoggerLocked();
    }
  }

  private static Logger ensureJulDispatcherLoggerLocked() {
    Logger cached = dispatcherJvmLogger;
    if (cached != null) {
      return cached;
    }
    Pointer np =
        ArenaBindings.lib().arena_dispatcher_default_logging_target_logger_name_utf8();
    String name =
        np != null
            ? np.getString(0, java.nio.charset.StandardCharsets.UTF_8.name())
            : "arena.rust.dispatcher";
    cached = Logger.getLogger(name);
    dispatcherJvmLogger = cached;
    return cached;
  }

  private static Level julFloorForArena(ArenaLogLevel arenaLogLevel) {
    if (arenaLogLevel == ArenaLogLevel.ERROR) {
      return Level.SEVERE;
    }
    if (arenaLogLevel == ArenaLogLevel.WARN) {
      return Level.WARNING;
    }
    if (arenaLogLevel == ArenaLogLevel.INFO) {
      return Level.INFO;
    }
    if (arenaLogLevel == ArenaLogLevel.DEBUG) {
      return Level.FINE;
    }
    if (arenaLogLevel == ArenaLogLevel.TRACE) {
      return Level.FINEST;
    }
    return Level.INFO;
  }

  private static Level julPublishLevelForDispatcherLoggingTarget(
      int dispatcherLoggingTargetPublishLevel) {
    if (dispatcherLoggingTargetPublishLevel == ArenaLogLevel.ERROR.code()) {
      return Level.SEVERE;
    }
    if (dispatcherLoggingTargetPublishLevel == ArenaLogLevel.WARN.code()) {
      return Level.WARNING;
    }
    if (dispatcherLoggingTargetPublishLevel == ArenaLogLevel.INFO.code()) {
      return Level.INFO;
    }
    if (dispatcherLoggingTargetPublishLevel == ArenaLogLevel.DEBUG.code()) {
      return Level.FINE;
    }
    if (dispatcherLoggingTargetPublishLevel == ArenaLogLevel.TRACE.code()) {
      return Level.FINEST;
    }
    return Level.INFO;
  }
}
