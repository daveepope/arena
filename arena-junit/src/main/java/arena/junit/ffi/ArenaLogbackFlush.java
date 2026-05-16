package arena.junit.ffi;

import java.io.Flushable;
import java.io.IOException;
import java.lang.reflect.Method;
import java.util.Iterator;
import org.slf4j.LoggerFactory;

public final class ArenaLogbackFlush {

  private ArenaLogbackFlush() {}

  public static void flushIfPresent() {
    try {
      Object context = LoggerFactory.getILoggerFactory();
      if (context == null) {
        return;
      }
      if (!"ch.qos.logback.classic.LoggerContext".equals(context.getClass().getName())) {
        return;
      }
      Method getLoggerList = context.getClass().getMethod("getLoggerList");
      Iterable<?> loggers = (Iterable<?>) getLoggerList.invoke(context);
      for (Object logger : loggers) {
        Method iterApp = logger.getClass().getMethod("iteratorForAppenders");
        Iterator<?> it = (Iterator<?>) iterApp.invoke(logger);
        while (it.hasNext()) {
          Object appender = it.next();
          flushAppenderOutputStream(appender);
        }
      }
    } catch (ReflectiveOperationException ignored) {
    } catch (ClassCastException ignored) {
    }
    System.out.flush();
    System.err.flush();
  }

  private static void flushAppenderOutputStream(Object appender) {
    try {
      Method m = null;
      for (Class<?> c = appender.getClass(); c != null; c = c.getSuperclass()) {
        try {
          m = c.getDeclaredMethod("getOutputStream");
          m.setAccessible(true);
          break;
        } catch (NoSuchMethodException e) {
        }
      }
      if (m == null) {
        return;
      }
      Object os = m.invoke(appender);
      if (os instanceof Flushable) {
        try {
          ((Flushable) os).flush();
        } catch (IOException ignored) {
        }
      }
    } catch (ReflectiveOperationException ignored) {
    }
  }
}
