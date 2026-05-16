package arena.junit.ffi;

import java.lang.reflect.Method;
import org.slf4j.Logger;

final class ArenaSlf4jLogbackAlign {

  private ArenaSlf4jLogbackAlign() {}

  static void alignSlf4jLoggerWithArenaLogLevel(Logger logger, ArenaLogLevel arenaLogLevel) {
    try {
      Class<?> lbLogger = Class.forName("ch.qos.logback.classic.Logger");
      if (!lbLogger.isInstance(logger)) {
        return;
      }
      Class<?> lbLevel = Class.forName("ch.qos.logback.classic.Level");
      Object level = lbLevel.getField(arenaLogLevel.name()).get(null);
      Object cast = lbLogger.cast(logger);
      lbLogger.getMethod("setLevel", lbLevel).invoke(cast, level);
    } catch (ReflectiveOperationException ignored) {
    }
  }
}
