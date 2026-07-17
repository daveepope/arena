package arena.junit;

import arena.junit.ffi.ArenaLogLevel;
import arena.junit.match.ArenaMatchPiece;
import arena.junit.match.Match;
import arena.junit.match.MatchBuilder;
import arena.junit.playbook.Playbook;

import java.lang.annotation.Annotation;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.util.List;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.atomic.AtomicBoolean;

import org.junit.jupiter.api.extension.AfterAllCallback;
import org.junit.jupiter.api.extension.BeforeAllCallback;
import org.junit.jupiter.api.extension.ExtensionContext;
import org.junit.jupiter.api.extension.ParameterContext;
import org.junit.jupiter.api.extension.ParameterResolutionException;
import org.junit.jupiter.api.extension.ParameterResolver;
import org.slf4j.Logger;

public final class ArenaExtension implements BeforeAllCallback, AfterAllCallback, ParameterResolver {

  private static final ConcurrentHashMap<Class<?>, CachedArena> CACHE = new ConcurrentHashMap<>();
  private static final ConcurrentHashMap<Class<?>, OpenArena> SHUTDOWN_ARENAS =
      new ConcurrentHashMap<>();
  private static final AtomicBoolean SHUTDOWN_HOOK_REGISTERED = new AtomicBoolean(false);

  public static OpenArena openArenaFor(Class<?> testClass) {
    return openArenaForRoot(topologyRoot(testClass));
  }

  @Override
  public void beforeAll(ExtensionContext context) {
    Class<?> root = topologyRoot(context.getRequiredTestClass());
    CACHE.compute(
        root,
        (key, existing) -> {
          CachedArena cached = existing != null ? existing : buildAndOpen(root);
          cached.refs++;
          return cached;
        });
  }

  @Override
  public void afterAll(ExtensionContext context) {
    Class<?> root = topologyRoot(context.getRequiredTestClass());
    CACHE.compute(
        root,
        (key, existing) -> {
          if (existing == null) {
            return null;
          }
          existing.refs--;
          if (existing.refs > 0) {
            return existing;
          }
          invokeLifecycleMethod(root, ArenaBeforeClose.class, existing.openArena);
          existing.openArena.close();
          SHUTDOWN_ARENAS.remove(root);
          return null;
        });
  }

  @Override
  public boolean supportsParameter(ParameterContext parameterContext, ExtensionContext extensionContext) {
    Class<?> paramType = parameterContext.getParameter().getType();
    if (OpenArena.class.equals(paramType)) {
      return true;
    }
    Class<?> root = topologyRoot(extensionContext.getRequiredTestClass());
    return findMatchingField(root, paramType) != null;
  }

  @Override
  public Object resolveParameter(ParameterContext parameterContext, ExtensionContext extensionContext) {
    Class<?> root = topologyRoot(extensionContext.getRequiredTestClass());
    Class<?> paramType = parameterContext.getParameter().getType();
    if (OpenArena.class.equals(paramType)) {
      return openArenaForRoot(root);
    }
    Field field = findMatchingField(root, paramType);
    return readStatic(field, root);
  }

  private static Class<?> topologyRoot(Class<?> testClass) {
    Class<?> root = null;
    for (Class<?> current = testClass;
        current != null && current != Object.class;
        current = current.getSuperclass()) {
      if (declaresArenaFields(current)) {
        root = current;
      }
    }
    if (root == null) {
      throw new IllegalStateException(
          "@Arena requires at least one @ArenaDependency, @ArenaComponent, or @ArenaPlaybook "
              + "field on "
              + testClass.getName()
              + " or a superclass");
    }
    return root;
  }

  private static MatchBuilder buildMatchBuilder(Class<?> root) {
    MatchBuilder matchBuilder = new MatchBuilder(root.getSimpleName());
    for (Field field : root.getDeclaredFields()) {
      if (field.isAnnotationPresent(ArenaDependency.class)) {
        matchBuilder.addDependency((ArenaMatchPiece) readStatic(field, root));
      } else if (field.isAnnotationPresent(ArenaComponent.class)) {
        matchBuilder.addComponent((ArenaMatchPiece) readStatic(field, root));
      } else if (field.isAnnotationPresent(ArenaPlaybook.class)) {
        ArenaPlaybook annotation = field.getAnnotation(ArenaPlaybook.class);
        matchBuilder.registerPlaybook(
            (Playbook) readStatic(field, root), annotation.execOnDependencyStart());
      }
    }
    return matchBuilder;
  }

  private static ClosedArena closedArenaFor(Class<?> root, Match match) {
    Field loggerField = findLoggerField(root);
    if (loggerField == null) {
      return new ClosedArena(root.getSimpleName(), List.of(match));
    }
    Logger logger = (Logger) readStatic(loggerField, root);
    ArenaLogLevel level = loggerField.getAnnotation(ArenaLogger.class).level();
    return new ClosedArena(root.getSimpleName(), List.of(match), level, logger);
  }

  private static Field findLoggerField(Class<?> root) {
    Field found = null;
    for (Field field : root.getDeclaredFields()) {
      if (field.isAnnotationPresent(ArenaLogger.class)) {
        if (found != null) {
          throw new IllegalStateException("@Arena: multiple @ArenaLogger fields on " + root.getName());
        }
        found = field;
      }
    }
    return found;
  }

  private static Method findLifecycleMethod(Class<?> root, Class<? extends Annotation> annotationType) {
    Method found = null;
    for (Method method : root.getDeclaredMethods()) {
      if (method.isAnnotationPresent(annotationType)) {
        if (found != null) {
          throw new IllegalStateException(
              "@Arena: multiple " + annotationType.getSimpleName() + " methods on " + root.getName());
        }
        found = method;
      }
    }
    return found;
  }

  private static Field findMatchingField(Class<?> root, Class<?> paramType) {
    Field match = null;
    for (Field field : root.getDeclaredFields()) {
      boolean managed =
          field.isAnnotationPresent(ArenaDependency.class)
              || field.isAnnotationPresent(ArenaComponent.class);
      if (managed && paramType.isAssignableFrom(field.getType())) {
        if (match != null) {
          throw new ParameterResolutionException(
              "@Arena: multiple fields of type " + paramType.getName() + " on " + root.getName());
        }
        match = field;
      }
    }
    return match;
  }

  private static boolean declaresArenaFields(Class<?> type) {
    for (Field field : type.getDeclaredFields()) {
      if (field.isAnnotationPresent(ArenaDependency.class)
          || field.isAnnotationPresent(ArenaComponent.class)
          || field.isAnnotationPresent(ArenaPlaybook.class)) {
        return true;
      }
    }
    return false;
  }

  private static Object readStatic(Field field, Class<?> root) {
    if (!Modifier.isStatic(field.getModifiers())) {
      throw new IllegalStateException(
          "@Arena fields must be static: " + root.getName() + "." + field.getName());
    }
    field.setAccessible(true);
    try {
      return field.get(null);
    } catch (IllegalAccessException e) {
      throw new IllegalStateException(
          "@Arena: failed to read field " + root.getName() + "." + field.getName(), e);
    }
  }

  private static OpenArena openArenaForRoot(Class<?> root) {
    CachedArena cached = CACHE.get(root);
    if (cached == null) {
      throw new IllegalStateException(
          "@Arena: no open arena for " + root.getName() + " (beforeAll has not run yet)");
    }
    return cached.openArena;
  }

  private static CachedArena buildAndOpen(Class<?> root) {
    Match match = buildMatchBuilder(root).build();
    ClosedArena closedArena = closedArenaFor(root, match);
    OpenArena openArena;
    try {
      openArena = closedArena.open();
    } catch (Exception e) {
      throw new IllegalStateException("@Arena: failed to open arena for " + root.getName(), e);
    }
    registerShutdownHookOnce();
    SHUTDOWN_ARENAS.put(root, openArena);
    invokeLifecycleMethod(root, ArenaAfterOpen.class, openArena);
    return new CachedArena(openArena);
  }

  private static void invokeLifecycleMethod(
      Class<?> root, Class<? extends Annotation> annotationType, OpenArena openArena) {
    Method method = findLifecycleMethod(root, annotationType);
    if (method == null) {
      return;
    }
    if (!Modifier.isStatic(method.getModifiers())) {
      throw new IllegalStateException(
          annotationType.getSimpleName()
              + " method must be static: "
              + root.getName()
              + "."
              + method.getName());
    }
    method.setAccessible(true);
    try {
      if (method.getParameterCount() == 0) {
        method.invoke(null);
      } else {
        method.invoke(null, openArena);
      }
    } catch (ReflectiveOperationException e) {
      throw new IllegalStateException(
          "@Arena: failed to invoke " + annotationType.getSimpleName() + " method " + method.getName(),
          e);
    }
  }

  private static void registerShutdownHookOnce() {
    if (!SHUTDOWN_HOOK_REGISTERED.compareAndSet(false, true)) {
      return;
    }
    Runtime.getRuntime()
        .addShutdownHook(
            new Thread(
                () -> {
                  for (OpenArena openArena : SHUTDOWN_ARENAS.values()) {
                    if (openArena != null) {
                      openArena.close();
                    }
                  }
                  SHUTDOWN_ARENAS.clear();
                },
                "arena-junit-shared-arena-shutdown"));
  }

  private static final class CachedArena {
    final OpenArena openArena;
    int refs;

    CachedArena(OpenArena openArena) {
      this.openArena = openArena;
    }
  }
}
