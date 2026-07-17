package arena.junit;

import arena.junit.ffi.ArenaLogLevel;
import java.lang.annotation.ElementType;
import java.lang.annotation.Retention;
import java.lang.annotation.RetentionPolicy;
import java.lang.annotation.Target;

@Target(ElementType.FIELD)
@Retention(RetentionPolicy.RUNTIME)
public @interface ArenaLogger {
  ArenaLogLevel level() default ArenaLogLevel.INFO;
}
