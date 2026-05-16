package arena.junit.ffi;

import java.lang.ProcessHandle;
import java.lang.reflect.Method;
import org.slf4j.Logger;

final class ArenaSlf4jDispatcherStderrPublication {

  static final String STDERR_APPENDER_NAME = "arena.stderr.dispatcher";

  private final Class<?> lbLoggerCls;
  private final Object lb;
  private final boolean savedAdditive;
  private final Object savedLevel;

  private ArenaSlf4jDispatcherStderrPublication(
      Class<?> lbLoggerCls,
      Object lb,
      boolean savedAdditive,
      Object savedLevel) {
    this.lbLoggerCls = lbLoggerCls;
    this.lb = lb;
    this.savedAdditive = savedAdditive;
    this.savedLevel = savedLevel;
  }

  static ArenaSlf4jDispatcherStderrPublication installIfApplicable(
      Logger logger, ArenaLogLevel arenaLogLevel) {
    try {
      Class<?> lbLoggerCls = Class.forName("ch.qos.logback.classic.Logger");
      if (!lbLoggerCls.isInstance(logger)) {
        return null;
      }
      Object lb = lbLoggerCls.cast(logger);

      Method isAdditive = lbLoggerCls.getMethod("isAdditive");
      boolean prevAdditive = (Boolean) isAdditive.invoke(lb);
      Method getLevel = lbLoggerCls.getMethod("getLevel");
      Object prevLevel = getLevel.invoke(lb);

      ArenaSlf4jLogbackAlign.alignSlf4jLoggerWithArenaLogLevel(logger, arenaLogLevel);

      Method getAppender = lbLoggerCls.getMethod("getAppender", String.class);
      if (getAppender.invoke(lb, STDERR_APPENDER_NAME) == null) {
        appendStderrConsoleAppender(lb);
      }

      Method setAdditive = lbLoggerCls.getMethod("setAdditive", boolean.class);
      setAdditive.invoke(lb, false);

      return new ArenaSlf4jDispatcherStderrPublication(
          lbLoggerCls, lb, prevAdditive, prevLevel);
    } catch (ReflectiveOperationException | ClassCastException ignored) {
      return null;
    }
  }

  void restore() {
    try {
      Method getAppender = lbLoggerCls.getMethod("getAppender", String.class);
      Object app = getAppender.invoke(lb, STDERR_APPENDER_NAME);
      if (app != null) {
        Method detach = lbLoggerCls.getMethod("detachAppender", String.class);
        detach.invoke(lb, STDERR_APPENDER_NAME);
        Method stop = app.getClass().getMethod("stop");
        stop.invoke(app);
      }
      Class<?> lbLevel = Class.forName("ch.qos.logback.classic.Level");
      Method setLevel = lbLoggerCls.getMethod("setLevel", lbLevel);
      setLevel.invoke(lb, savedLevel);
      Method setAdditive = lbLoggerCls.getMethod("setAdditive", boolean.class);
      setAdditive.invoke(lb, savedAdditive);
    } catch (ReflectiveOperationException ignored) {
    }
  }

  private static void appendStderrConsoleAppender(Object lbLogger)
      throws ReflectiveOperationException {
    Class<?> lbCls = lbLogger.getClass();
    Method getContext = lbCls.getMethod("getLoggerContext");
    Object ctx = getContext.invoke(lbLogger);

    Class<?> contextIface = Class.forName("ch.qos.logback.core.Context");
    boolean pidProperty = false;
    try {
      Method putProperty = ctx.getClass().getMethod("putProperty", String.class, String.class);
      putProperty.invoke(
          ctx, "pid", Long.toString(ProcessHandle.current().pid()));
      pidProperty = true;
    } catch (ReflectiveOperationException ignored) {
    }

    Class<?> patternEncoderCls =
        Class.forName("ch.qos.logback.classic.encoder.PatternLayoutEncoder");
    Object encoder = patternEncoderCls.getConstructor().newInstance();
    patternEncoderCls.getMethod("setContext", contextIface).invoke(encoder, ctx);
    String pattern =
        pidProperty
            ? "%d{HH:mm:ss} [%property{pid}] %-5level %logger - %msg%n"
            : "%d{HH:mm:ss} [%thread] %-5level %logger - %msg%n";
    patternEncoderCls.getMethod("setPattern", String.class).invoke(encoder, pattern);
    patternEncoderCls.getMethod("start").invoke(encoder);

    Class<?> consoleAppenderCls = Class.forName("ch.qos.logback.core.ConsoleAppender");
    Object appender = consoleAppenderCls.getConstructor().newInstance();
    consoleAppenderCls.getMethod("setContext", contextIface).invoke(appender, ctx);
    consoleAppenderCls.getMethod("setName", String.class).invoke(appender, STDERR_APPENDER_NAME);
    Class<?> encoderSuper = Class.forName("ch.qos.logback.core.encoder.Encoder");
    consoleAppenderCls.getMethod("setEncoder", encoderSuper).invoke(appender, encoder);
    consoleAppenderCls.getMethod("setTarget", String.class).invoke(appender, "System.err");
    consoleAppenderCls.getMethod("start").invoke(appender);

    Method addAppender = lbCls.getMethod("addAppender", Class.forName("ch.qos.logback.core.Appender"));
    addAppender.invoke(lbLogger, appender);
  }
}
