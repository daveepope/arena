package arena.junit;

import arena.junit.ffi.ArenaLogLevel;
import arena.junit.match.ArenaRunnableComponent;
import arena.junit.match.ArenaRunnableDependency;
import arena.junit.match.Match;
import arena.junit.match.MatchBuilder;
import arena.junit.playbook.Playbook;

import java.lang.annotation.Annotation;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.lang.reflect.Modifier;
import java.util.ArrayList;
import java.util.HashSet;
import java.util.List;
import java.util.Set;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.atomic.AtomicBoolean;

import org.junit.jupiter.api.extension.AfterAllCallback;
import org.junit.jupiter.api.extension.BeforeAllCallback;
import org.junit.jupiter.api.extension.ExtensionContext;
import org.junit.jupiter.api.extension.ParameterContext;
import org.junit.jupiter.api.extension.ParameterResolutionException;
import org.junit.jupiter.api.extension.ParameterResolver;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

public final class ArenaExtension implements BeforeAllCallback, AfterAllCallback, ParameterResolver {

  private static final Logger LOG = LoggerFactory.getLogger(ArenaExtension.class);
  private static final ConcurrentHashMap<Class<?>, CachedArena> CACHE = new ConcurrentHashMap<>();
  private static final ConcurrentHashMap<Class<?>, OpenArena> SHUTDOWN_ARENAS =
      new ConcurrentHashMap<>();
  private static final ConcurrentHashMap<Class<?>, Class<?>> TOPOLOGY_ROOT_CACHE =
      new ConcurrentHashMap<>();
  private static final Set<Class<?>> WARNED_MISSING_SELECT_CLASSES =
      ConcurrentHashMap.newKeySet();
  private static final AtomicBoolean SHUTDOWN_HOOK_REGISTERED = new AtomicBoolean(false);
  private static final Class<? extends Annotation> SELECT_CLASSES_ANNOTATION_TYPE =
      resolveSelectClassesAnnotationType();

  public static OpenArena openArenaFor(Class<?> testClass) {
    return openArenaForRoot(topologyRoot(testClass));
  }

  @Override
  public void beforeAll(ExtensionContext context) {
    Class<?> testClass = context.getRequiredTestClass();
    Class<?> root = topologyRoot(testClass);
    warnIfExplicitRootMissingSelectClasses(testClass, root);
    CachedArena cached =
        CACHE.compute(
            root,
            (key, existing) -> {
              CachedArena current = existing != null ? existing : buildOrCacheFailure(root);
              if (current.expectedSuiteMembers == null) {
                current.refs++;
              }
              return current;
            });
    if (cached.failure != null) {
      throw cached.failure;
    }
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
          boolean shouldClose;
          if (existing.expectedSuiteMembers != null) {
            existing.completed++;
            shouldClose = existing.completed >= existing.expectedSuiteMembers;
          } else {
            existing.refs--;
            shouldClose = existing.refs <= 0;
          }
          if (!shouldClose) {
            return existing;
          }
          if (existing.failure == null) {
            invokeLifecycleMethod(root, ArenaBeforeClose.class, existing.openArena);
            existing.openArena.close();
            SHUTDOWN_ARENAS.remove(root);
          }
          return null;
        });
  }

  private static void warnIfExplicitRootMissingSelectClasses(Class<?> testClass, Class<?> root) {
    Class<?> explicit = explicitRoot(testClass);
    if (explicit == null || selectClassesValue(root) != null) {
      return;
    }
    if (WARNED_MISSING_SELECT_CLASSES.add(root)) {
      LOG.warn(
          "@Arena({}.class) on {} has no @Suite @SelectClasses; cross-class close timing "
              + "falls back to reference counting, which does not correctly share an arena "
              + "across classes run sequentially (wrap the shared members in a real "
              + "@Suite/@SelectClasses to get deterministic sharing)",
          explicit.getSimpleName(),
          testClass.getName());
    }
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
    return TOPOLOGY_ROOT_CACHE.computeIfAbsent(testClass, ArenaExtension::resolveTopologyRoot);
  }

  private static Class<?> resolveTopologyRoot(Class<?> testClass) {
    Class<?> explicit = explicitRoot(testClass);
    Class<?> searchStart = explicit != null ? explicit : testClass;
    Class<?> root = null;
    for (Class<?> current = searchStart;
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
              + searchStart.getName()
              + (explicit != null
                  ? " (referenced by @Arena(" + explicit.getSimpleName() + ".class) on " + testClass.getName() + ")"
                  : " or a superclass"));
    }
    return root;
  }

  private static Class<?> explicitRoot(Class<?> testClass) {
    for (Class<?> current = testClass;
        current != null && current != Object.class;
        current = current.getSuperclass()) {
      Arena annotation = current.getDeclaredAnnotation(Arena.class);
      if (annotation != null && annotation.value() != Void.class) {
        return annotation.value();
      }
    }
    return null;
  }

  private static MatchBuild buildMatchBuilder(Class<?> root) {
    MatchBuilder matchBuilder = new MatchBuilder(root.getSimpleName());
    List<String> dependencyIds = new ArrayList<>();
    List<String> componentIds = new ArrayList<>();
    for (Field field : root.getDeclaredFields()) {
      if (field.isAnnotationPresent(ArenaDependency.class)) {
        ArenaRunnableDependency piece = (ArenaRunnableDependency) readStatic(field, root);
        matchBuilder.addDependency(piece);
        if (field.getAnnotation(ArenaDependency.class).logs()) {
          dependencyIds.add(piece.forFfi().get("identifier").asText());
        }
      } else if (field.isAnnotationPresent(ArenaComponent.class)) {
        ArenaRunnableComponent piece = (ArenaRunnableComponent) readStatic(field, root);
        matchBuilder.addComponent(piece);
        if (field.getAnnotation(ArenaComponent.class).logs()) {
          componentIds.add(piece.forFfi().get("identifier").asText());
        }
      } else if (field.isAnnotationPresent(ArenaPlaybook.class)) {
        ArenaPlaybook annotation = field.getAnnotation(ArenaPlaybook.class);
        matchBuilder.registerPlaybook(
            (Playbook) readStatic(field, root), annotation.execOnDependencyStart());
      }
    }
    return new MatchBuild(matchBuilder, new LogIdentifiers(dependencyIds, componentIds));
  }

  private static ClosedArena closedArenaFor(Class<?> root, Match match, LogIdentifiers logIdentifiers) {
    Field loggerField = findLoggerField(root);
    ArenaLogLevel level =
        loggerField != null
            ? loggerField.getAnnotation(ArenaLogger.class).level()
            : ArenaLogLevel.INFO;
    Logger logger = loggerField != null ? (Logger) readStatic(loggerField, root) : null;
    return new ClosedArena(
        root.getSimpleName(),
        List.of(match),
        level,
        logger,
        logIdentifiers.dependencyIds(),
        logIdentifiers.componentIds());
  }

  private record MatchBuild(MatchBuilder matchBuilder, LogIdentifiers logIdentifiers) {}

  private record LogIdentifiers(List<String> dependencyIds, List<String> componentIds) {}

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

  private static CachedArena buildOrCacheFailure(Class<?> root) {
    try {
      return buildAndOpen(root);
    } catch (RuntimeException e) {
      return new CachedArena(e, expectedSuiteMembers(root));
    }
  }

  private static CachedArena buildAndOpen(Class<?> root) {
    MatchBuild matchBuild = buildMatchBuilder(root);
    Match match = matchBuild.matchBuilder().build();
    ClosedArena closedArena = closedArenaFor(root, match, matchBuild.logIdentifiers());
    OpenArena openArena;
    try {
      openArena = closedArena.open();
    } catch (Exception e) {
      throw new IllegalStateException("@Arena: failed to open arena for " + root.getName(), e);
    }
    registerShutdownHookOnce();
    SHUTDOWN_ARENAS.put(root, openArena);
    invokeLifecycleMethod(root, ArenaAfterOpen.class, openArena);
    return new CachedArena(openArena, expectedSuiteMembers(root));
  }

  private static Class<? extends Annotation> resolveSelectClassesAnnotationType() {
    try {
      return Class.forName("org.junit.platform.suite.api.SelectClasses").asSubclass(Annotation.class);
    } catch (ClassNotFoundException e) {
      return null;
    }
  }

  private static Class<?>[] selectClassesValue(Class<?> root) {
    if (SELECT_CLASSES_ANNOTATION_TYPE == null) {
      return null;
    }
    Annotation selectClasses = root.getAnnotation(SELECT_CLASSES_ANNOTATION_TYPE);
    if (selectClasses == null) {
      return null;
    }
    try {
      return (Class<?>[]) SELECT_CLASSES_ANNOTATION_TYPE.getMethod("value").invoke(selectClasses);
    } catch (ReflectiveOperationException e) {
      return null;
    }
  }

  private static Integer expectedSuiteMembers(Class<?> root) {
    Class<?>[] candidates = selectClassesValue(root);
    if (candidates == null) {
      return null;
    }
    Set<Class<?>> members = new HashSet<>();
    for (Class<?> candidate : candidates) {
      if (root.equals(explicitRoot(candidate))) {
        members.add(candidate);
      }
    }
    return members.isEmpty() ? null : members.size();
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
    final RuntimeException failure;
    final Integer expectedSuiteMembers;
    int refs;
    int completed;

    CachedArena(OpenArena openArena, Integer expectedSuiteMembers) {
      this.openArena = openArena;
      this.failure = null;
      this.expectedSuiteMembers = expectedSuiteMembers;
    }

    CachedArena(RuntimeException failure, Integer expectedSuiteMembers) {
      this.openArena = null;
      this.failure = failure;
      this.expectedSuiteMembers = expectedSuiteMembers;
    }
  }
}
