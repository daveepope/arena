package arena.junit.playbook;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertSame;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import arena.junit.ArenaExtension;
import arena.junit.OpenArena;
import arena.junit.match.ArenaMatchPiece;
import arena.junit.match.Match;
import arena.junit.match.MatchBuilder;
import com.fasterxml.jackson.databind.node.JsonNodeFactory;
import com.fasterxml.jackson.databind.node.ObjectNode;
import com.sun.jna.Pointer;
import java.lang.reflect.Constructor;
import java.lang.reflect.Field;
import java.lang.reflect.InvocationTargetException;
import java.lang.reflect.Method;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.util.Set;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.extension.ExecutableInvoker;
import org.junit.jupiter.api.extension.ExtensionContext;
import org.junit.jupiter.api.extension.ParameterContext;
import org.junit.jupiter.api.extension.TestInstances;
import org.junit.jupiter.api.parallel.ExecutionMode;

final class PlaybookInvocationExtensionUnitTest {

  private final PlaybookInvocationExtension extension = new PlaybookInvocationExtension();

  @Test
  void supportsParameter_activeHttpPlaybook_returnsTrue() throws Exception {
    ParameterContext parameterContext = parameterContext(ActiveHttpPlaybook.class);
    assertTrue(extension.supportsParameter(parameterContext, extensionContext()));
  }

  @Test
  void supportsParameter_unrelatedType_returnsFalse() throws Exception {
    ParameterContext parameterContext = parameterContext(String.class);
    assertFalse(extension.supportsParameter(parameterContext, extensionContext()));
  }

  @Test
  void resolveParameter_singleActiveHttpPlaybook_returnsThatPlaybook() throws Exception {
    StubActiveHttpPlaybook active = new StubActiveHttpPlaybook();
    ExtensionContext context = extensionContextWithMethodScope(List.of(active));
    Object resolved =
        extension.resolveParameter(parameterContext(ActiveHttpPlaybook.class), context);
    assertSame(active, resolved);
  }

  @Test
  void resolveParameter_missingMethodScope_throwsIllegalStateException() throws Exception {
    ExtensionContext context = extensionContext();
    IllegalStateException error =
        assertThrows(
            IllegalStateException.class,
            () ->
                extension.resolveParameter(
                    parameterContext(ActiveHttpPlaybook.class), context));
    assertTrue(error.getMessage().contains("stacked @Playbook"));
  }

  @Test
  void resolveParameter_multipleActiveHttpPlaybooks_throwsIllegalStateException() throws Exception {
    ExtensionContext context =
        extensionContextWithMethodScope(
            List.of(new StubActiveHttpPlaybook(), new StubActiveHttpPlaybook()));
    IllegalStateException error =
        assertThrows(
            IllegalStateException.class,
            () ->
                extension.resolveParameter(
                    parameterContext(ActiveHttpPlaybook.class), context));
    assertTrue(error.getMessage().contains("exactly one ActiveHttpPlaybook"));
  }

  @Test
  void resolveParameter_noActiveHttpPlaybookInScope_throwsIllegalStateException() throws Exception {
    ExtensionContext context =
        extensionContextWithMethodScope(List.of(new StubNonHttpActivePlaybook()));
    IllegalStateException error =
        assertThrows(
            IllegalStateException.class,
            () ->
                extension.resolveParameter(
                    parameterContext(ActiveHttpPlaybook.class), context));
    assertTrue(error.getMessage().contains("exactly one ActiveHttpPlaybook"));
  }

  @Test
  void afterEach_withoutMethodScope_isNoOp() throws Exception {
    extension.afterEach(extensionContext());
  }

  @Test
  void beforeEach_withoutPlaybookAnnotations_isNoOp() throws Exception {
    extension.beforeEach(extensionContext());
  }

  @Test
  void beforeAll_withoutPlaybookAnnotations_isNoOp() throws Exception {
    extension.beforeAll(extensionContext());
  }

  @Test
  void afterAll_withoutClassScope_isNoOp() throws Exception {
    extension.afterAll(extensionContext());
  }

  @Test
  void beforeEach_methodPlaybookAnnotation_opensMethodScopeForParameterResolution() throws Exception {
    OpenArena arena = openArenaWithPlaybook(new UnitStubPlaybook(), false);
    seedOpenArena(MethodPlaybookHost.class, arena);
    Method method = MethodPlaybookHost.class.getDeclaredMethod("scopedTest");
    ExtensionContext context = methodContext(MethodPlaybookHost.class, method, classContext(MethodPlaybookHost.class));

    extension.beforeEach(context);

    Object resolved =
        extension.resolveParameter(parameterContext(ActiveHttpPlaybook.class), context);
    assertTrue(resolved instanceof ActiveHttpPlaybook);

    extension.afterEach(context);
  }

  @Test
  void beforeAll_classPlaybookAnnotation_opensClassScope() throws Exception {
    OpenArena arena = openArenaWithPlaybook(new UnitStubPlaybook(), false);
    seedOpenArena(ClassPlaybookHost.class, arena);
    ExtensionContext context = classContext(ClassPlaybookHost.class);

    extension.beforeAll(context);

    assertNotNull(classScope(context));
    extension.afterAll(context);
  }

  @Test
  void beforeEach_classPlaybookWithoutMethodAnnotation_reusesClassScope() throws Exception {
    OpenArena arena = openArenaWithPlaybook(new UnitStubPlaybook(), false);
    seedOpenArena(ClassPlaybookHost.class, arena);
    ExtensionContext classCtx = classContext(ClassPlaybookHost.class);
    extension.beforeAll(classCtx);
    Method method = ClassPlaybookHost.class.getDeclaredMethod("plainTest");
    ExtensionContext methodCtx = methodContext(ClassPlaybookHost.class, method, classCtx);

    extension.beforeEach(methodCtx);

    assertNotNull(classScope(classCtx));
    extension.afterEach(methodCtx);
    extension.afterAll(classCtx);
  }

  @Test
  void resolveOpenArena_noArenaAnnotatedClass_throwsIllegalStateException() throws Exception {
    Method method = MissingExtensionHost.class.getDeclaredMethod("scopedTest");
    ExtensionContext context =
        methodContext(
            MissingExtensionHost.class,
            method,
            rootContext(MissingExtensionHost.class, new MapExtensionStore()));
    IllegalStateException error =
        assertThrows(IllegalStateException.class, () -> extension.beforeEach(context));
    assertTrue(error.getMessage().contains("@ArenaDependency"));
  }

  @Test
  void resolveOpenArena_topologyNotYetOpened_throwsIllegalStateException() throws Exception {
    Method method = NotYetOpenedHost.class.getDeclaredMethod("scopedTest");
    ExtensionContext context =
        methodContext(
            NotYetOpenedHost.class,
            method,
            rootContext(NotYetOpenedHost.class, new MapExtensionStore()));
    IllegalStateException error =
        assertThrows(IllegalStateException.class, () -> extension.beforeEach(context));
    assertTrue(error.getMessage().contains("beforeAll has not run yet"));
  }

  @Test
  void openScope_unregisteredPlaybook_throwsIllegalStateException() throws Exception {
    OpenArena arena = openArenaWithPlaybook(new UnitStubPlaybook(), false);
    Method openScope =
        PlaybookInvocationExtension.class.getDeclaredMethod(
            "openScope", OpenArena.class, Class[].class);
    openScope.setAccessible(true);
    Exception error =
        assertThrows(
            Exception.class,
            () ->
                openScope.invoke(
                    null, arena, new Class<?>[] {UnregisteredPlaybook.class}));
    assertTrue(error instanceof IllegalStateException || error.getCause() instanceof IllegalStateException);
    String message =
        error instanceof IllegalStateException
            ? error.getMessage()
            : error.getCause().getMessage();
    assertTrue(message.contains("no playbook of class"));
  }

  @Test
  void openScope_execOnDependencyStartPlaybook_throwsIllegalStateException() throws Exception {
    OpenArena arena = openArenaWithPlaybook(new UnitStubPlaybook(), true);
    Method openScope =
        PlaybookInvocationExtension.class.getDeclaredMethod(
            "openScope", OpenArena.class, Class[].class);
    openScope.setAccessible(true);
    Exception error =
        assertThrows(
            Exception.class,
            () -> openScope.invoke(null, arena, new Class<?>[] {UnitStubPlaybook.class}));
    assertTrue(error instanceof IllegalStateException || error.getCause() instanceof IllegalStateException);
    String message =
        error instanceof IllegalStateException
            ? error.getMessage()
            : error.getCause().getMessage();
    assertTrue(message.contains("execOnDependencyStart=true"));
  }

  @Test
  void openScope_registeredPlaybook_returnsActivePlaybook() throws Exception {
    OpenArena arena = openArenaWithPlaybook(new UnitStubPlaybook(), false);
    Method openScope =
        PlaybookInvocationExtension.class.getDeclaredMethod(
            "openScope", OpenArena.class, Class[].class);
    openScope.setAccessible(true);
    Object scope =
        openScope.invoke(null, arena, new Class<?>[] {UnitStubPlaybook.class});
    assertNotNull(scope);
    Method actives =
        scope.getClass().getDeclaredMethod("actives");
    actives.setAccessible(true);
    @SuppressWarnings("unchecked")
    List<ActivePlaybook> opened = (List<ActivePlaybook>) actives.invoke(scope);
    assertEquals(1, opened.size());
    Method finish = scope.getClass().getDeclaredMethod("finish");
    finish.setAccessible(true);
    finish.invoke(scope);
  }

  @Test
  void openScope_managedPlaybook_defersRunToFinish() throws Exception {
    List<String> calls = new ArrayList<>();
    ManagedStubPlaybook managed = new ManagedStubPlaybook("cleanup", calls, false);
    OpenArena arena = openArenaWithPlaybooks(managed);

    Object scope = invokeOpenScope(arena, ManagedStubPlaybook.class);
    assertEquals(List.of(), calls);
    assertEquals(0, activesOf(scope).size());

    invokeFinish(scope);
    assertEquals(List.of("cleanup"), calls);
  }

  @Test
  void openScope_unmanagedPlaybook_runsBeforeFinish() throws Exception {
    List<String> calls = new ArrayList<>();
    UnitStubPlaybook seed = new UnitStubPlaybook();
    OpenArena arena = openArenaWithPlaybooks(seed);

    Object scope = invokeOpenScope(arena, UnitStubPlaybook.class);
    assertEquals(1, activesOf(scope).size());

    invokeFinish(scope);
  }

  @Test
  void openScope_mixedStack_runsUnmanagedBeforeAndManagedAfter() throws Exception {
    List<String> calls = new ArrayList<>();
    ManagedOrderStubPlaybook managed = new ManagedOrderStubPlaybook("managed", calls, false);
    UnmanagedOrderStubPlaybook unmanaged = new UnmanagedOrderStubPlaybook("unmanaged", calls);
    Match match =
        new MatchBuilder("mixed-match")
            .registerPlaybook(managed, false)
            .registerPlaybook(unmanaged, false)
            .build();
    OpenArena arena = newOpenArena(List.of(match));

    Object scope = invokeOpenScope(arena, ManagedOrderStubPlaybook.class, UnmanagedOrderStubPlaybook.class);
    assertEquals(List.of("unmanaged"), calls);

    invokeFinish(scope);
    assertEquals(List.of("unmanaged", "managed"), calls);
  }

  @Test
  void openScope_managedPlaybookOverridingActivatesBeforeTest_runsBeforeFinish() throws Exception {
    List<String> calls = new ArrayList<>();
    ManagedStubPlaybook preconfigured = new ManagedStubPlaybook("preconfigured", calls, true);
    OpenArena arena = openArenaWithPlaybooks(preconfigured);

    Object scope = invokeOpenScope(arena, ManagedStubPlaybook.class);
    assertEquals(List.of("preconfigured"), calls);
    assertEquals(1, activesOf(scope).size());

    invokeFinish(scope);
    assertEquals(List.of("preconfigured"), calls);
  }

  @Test
  void finish_oneManagedPlaybookFails_stillRunsRemainingManagedPlaybooks() throws Exception {
    List<String> calls = new ArrayList<>();
    FailingManagedStubPlaybook failing = new FailingManagedStubPlaybook("failing", calls);
    ManagedStubPlaybook cleanup = new ManagedStubPlaybook("cleanup", calls, false);
    Match match =
        new MatchBuilder("resilience-match")
            .registerPlaybook(failing, false)
            .registerPlaybook(cleanup, false)
            .build();
    OpenArena arena = newOpenArena(List.of(match));

    Object scope = invokeOpenScope(arena, FailingManagedStubPlaybook.class, ManagedStubPlaybook.class);

    RuntimeException error = assertThrows(RuntimeException.class, () -> invokeFinish(scope));
    assertTrue(error.getMessage().contains("boom"));
    assertEquals(List.of("failing", "cleanup"), calls);
  }

  private static Object invokeOpenScope(OpenArena arena, Class<? extends Playbook>... classes)
      throws Exception {
    Method openScope =
        PlaybookInvocationExtension.class.getDeclaredMethod(
            "openScope", OpenArena.class, Class[].class);
    openScope.setAccessible(true);
    return openScope.invoke(null, arena, classes);
  }

  private static void invokeFinish(Object scope) throws Exception {
    Method finish = scope.getClass().getDeclaredMethod("finish");
    finish.setAccessible(true);
    try {
      finish.invoke(scope);
    } catch (InvocationTargetException e) {
      if (e.getCause() instanceof RuntimeException re) {
        throw re;
      }
      throw e;
    }
  }

  @SuppressWarnings("unchecked")
  private static List<ActivePlaybook> activesOf(Object scope) throws Exception {
    Method actives = scope.getClass().getDeclaredMethod("actives");
    actives.setAccessible(true);
    return (List<ActivePlaybook>) actives.invoke(scope);
  }

  private static OpenArena openArenaWithPlaybooks(Playbook playbook) throws Exception {
    Match match = new MatchBuilder("multi-match").registerPlaybook(playbook, false).build();
    return newOpenArena(List.of(match));
  }

  private static OpenArena newOpenArena(List<Match> matches) throws Exception {
    Constructor<OpenArena> constructor =
        OpenArena.class.getDeclaredConstructor(Pointer.class, long.class, List.class);
    constructor.setAccessible(true);
    return constructor.newInstance(new Pointer(1), 0L, matches);
  }

  static final class ManagedStubPlaybook implements ManagedPlaybook, PlaybookRegistration {
    private final String identifier;
    private final List<String> calls;
    private final boolean activatesBeforeTest;

    ManagedStubPlaybook(String identifier, List<String> calls, boolean activatesBeforeTest) {
      this.identifier = identifier;
      this.calls = calls;
      this.activatesBeforeTest = activatesBeforeTest;
    }

    @Override
    public String identifier() {
      return identifier;
    }

    @Override
    public boolean activatesBeforeTest() {
      return activatesBeforeTest;
    }

    @Override
    public ActivePlaybook run(OpenArena arena) {
      calls.add(identifier);
      return new StubQuietActivePlaybook();
    }

    @Override
    public ObjectNode forRegisteredFfi() {
      return JsonNodeFactory.instance.objectNode().put("kind", "http").put("identifier", identifier);
    }
  }

  static final class ManagedOrderStubPlaybook implements ManagedPlaybook, PlaybookRegistration {
    private final String identifier;
    private final List<String> calls;
    private final boolean activatesBeforeTest;

    ManagedOrderStubPlaybook(String identifier, List<String> calls, boolean activatesBeforeTest) {
      this.identifier = identifier;
      this.calls = calls;
      this.activatesBeforeTest = activatesBeforeTest;
    }

    @Override
    public String identifier() {
      return identifier;
    }

    @Override
    public boolean activatesBeforeTest() {
      return activatesBeforeTest;
    }

    @Override
    public ActivePlaybook run(OpenArena arena) {
      calls.add(identifier);
      return new StubQuietActivePlaybook();
    }

    @Override
    public ObjectNode forRegisteredFfi() {
      return JsonNodeFactory.instance.objectNode().put("kind", "http").put("identifier", identifier);
    }
  }

  static final class UnmanagedOrderStubPlaybook implements UnmanagedPlaybook, PlaybookRegistration {
    private final String identifier;
    private final List<String> calls;

    UnmanagedOrderStubPlaybook(String identifier, List<String> calls) {
      this.identifier = identifier;
      this.calls = calls;
    }

    @Override
    public String identifier() {
      return identifier;
    }

    @Override
    public ActivePlaybook run(OpenArena arena) {
      calls.add(identifier);
      return new StubQuietActivePlaybook();
    }

    @Override
    public ObjectNode forRegisteredFfi() {
      return JsonNodeFactory.instance.objectNode().put("kind", "http").put("identifier", identifier);
    }
  }

  static final class FailingManagedStubPlaybook implements ManagedPlaybook, PlaybookRegistration {
    private final String identifier;
    private final List<String> calls;

    FailingManagedStubPlaybook(String identifier, List<String> calls) {
      this.identifier = identifier;
      this.calls = calls;
    }

    @Override
    public String identifier() {
      return identifier;
    }

    @Override
    public ActivePlaybook run(OpenArena arena) {
      calls.add(identifier);
      throw new RuntimeException("boom");
    }

    @Override
    public ObjectNode forRegisteredFfi() {
      return JsonNodeFactory.instance.objectNode().put("kind", "http").put("identifier", identifier);
    }
  }

  private static ExtensionContext classContext(Class<?> testClass) {
    return rootContext(testClass, new MapExtensionStore());
  }

  private static ExtensionContext methodContext(
      Class<?> testClass, Method method, ExtensionContext parent) {
    return new MinimalExtensionContext(
        new MapExtensionStore(), Optional.of(method), Optional.of(parent), testClass);
  }

  private static ExtensionContext rootContext(Class<?> testClass, MapExtensionStore store) {
    return new MinimalExtensionContext(store, Optional.empty(), Optional.empty(), testClass);
  }

  private static Object classScope(ExtensionContext classContext) throws Exception {
    Field nsField = PlaybookInvocationExtension.class.getDeclaredField("NS");
    nsField.setAccessible(true);
    ExtensionContext.Namespace ns = (ExtensionContext.Namespace) nsField.get(null);
    Field keyField = PlaybookInvocationExtension.class.getDeclaredField("CLASS_SCOPE_KEY");
    keyField.setAccessible(true);
    String key = (String) keyField.get(null);
    return classContext.getStore(ns).get(key);
  }

  private static OpenArena openArenaWithPlaybook(UnitStubPlaybook playbook, boolean execOnStart)
      throws Exception {
    Match match = new MatchBuilder("unit-match").registerPlaybook(playbook, execOnStart).build();
    Constructor<OpenArena> constructor =
        OpenArena.class.getDeclaredConstructor(Pointer.class, long.class, List.class);
    constructor.setAccessible(true);
    return constructor.newInstance(new Pointer(1), 0L, List.of(match));
  }

  static final class TopologyMarker implements ArenaMatchPiece {
    @Override
    public ObjectNode forFfi() {
      return JsonNodeFactory.instance.objectNode();
    }
  }

  static final class MethodPlaybookHost {
    @arena.junit.ArenaDependency static final TopologyMarker TOPOLOGY = new TopologyMarker();

    @arena.junit.Playbook(UnitStubPlaybook.class)
    void scopedTest() {}
  }

  @arena.junit.Playbook(UnitStubPlaybook.class)
  static final class ClassPlaybookHost {
    @arena.junit.ArenaDependency static final TopologyMarker TOPOLOGY = new TopologyMarker();

    void plainTest() {}
  }

  static final class MissingExtensionHost {
    @arena.junit.Playbook(UnitStubPlaybook.class)
    void scopedTest() {}
  }

  static final class NotYetOpenedHost {
    @arena.junit.ArenaDependency static final TopologyMarker TOPOLOGY = new TopologyMarker();

    @arena.junit.Playbook(UnitStubPlaybook.class)
    void scopedTest() {}
  }

  static final class UnregisteredPlaybook implements UnmanagedPlaybook, PlaybookRegistration {
    @Override
    public String identifier() {
      return "missing";
    }

    @Override
    public ActivePlaybook run(OpenArena arena) {
      return new StubActiveHttpPlaybook();
    }

    @Override
    public ObjectNode forRegisteredFfi() {
      return JsonNodeFactory.instance.objectNode().put("kind", "http");
    }
  }

  static final class UnitStubPlaybook implements UnmanagedPlaybook, PlaybookRegistration {
    @Override
    public String identifier() {
      return "unit-stub";
    }

    @Override
    public ActivePlaybook run(OpenArena arena) {
      return new StubActiveHttpPlaybook();
    }

    @Override
    public ObjectNode forRegisteredFfi() {
      return JsonNodeFactory.instance.objectNode().put("kind", "http").put("identifier", identifier());
    }
  }

  private static void seedOpenArena(Class<?> hostClass, OpenArena openArena) throws Exception {
    Class<?> cachedArenaClass = Class.forName("arena.junit.ArenaExtension$CachedArena");
    Constructor<?> cachedArenaConstructor = cachedArenaClass.getDeclaredConstructor(OpenArena.class);
    cachedArenaConstructor.setAccessible(true);
    Object cachedArena = cachedArenaConstructor.newInstance(openArena);
    Field refsField = cachedArenaClass.getDeclaredField("refs");
    refsField.setAccessible(true);
    refsField.setInt(cachedArena, 1);
    Field cacheField = ArenaExtension.class.getDeclaredField("CACHE");
    cacheField.setAccessible(true);
    @SuppressWarnings("unchecked")
    Map<Class<?>, Object> cache = (Map<Class<?>, Object>) cacheField.get(null);
    cache.put(hostClass, cachedArena);
  }

  private static ExtensionContext extensionContext() {
    return rootContext(PlaybookInvocationExtensionUnitTest.class, new MapExtensionStore());
  }

  private static ParameterContext parameterContext(Class<?> parameterType) throws Exception {
    java.lang.reflect.Method method =
        ParameterHolder.class.getDeclaredMethod("accept", parameterType);
    return new ParameterContext() {
      @Override
      public java.lang.reflect.Parameter getParameter() {
        return method.getParameters()[0];
      }

      @Override
      public int getIndex() {
        return 0;
      }

      @Override
      public Optional<Object> getTarget() {
        return Optional.empty();
      }

      @Override
      public boolean isAnnotated(Class<? extends java.lang.annotation.Annotation> annotationType) {
        return false;
      }

      @Override
      public <A extends java.lang.annotation.Annotation> Optional<A> findAnnotation(
          Class<A> annotationType) {
        return Optional.empty();
      }

      @Override
      public <A extends java.lang.annotation.Annotation> List<A> findRepeatableAnnotations(
          Class<A> annotationType) {
        return List.of();
      }
    };
  }

  private static ExtensionContext extensionContextWithMethodScope(List<ActivePlaybook> actives)
      throws Exception {
    MapExtensionStore store = new MapExtensionStore();
    store.put(methodScopeKey(), newPlaybookScope(actives));
    return new MinimalExtensionContext(
        store,
        Optional.of(
            ParameterHolder.class.getDeclaredMethod("accept", ActiveHttpPlaybook.class)),
        Optional.empty(),
        PlaybookInvocationExtensionUnitTest.class);
  }

  private static Object newPlaybookScope(List<ActivePlaybook> actives) throws Exception {
    Class<?> scopeClass =
        Class.forName("arena.junit.playbook.PlaybookInvocationExtension$PlaybookScope");
    Constructor<?> constructor =
        scopeClass.getDeclaredConstructor(OpenArena.class, List.class, List.class);
    constructor.setAccessible(true);
    return constructor.newInstance(null, actives, List.of());
  }

  private static String methodScopeKey() throws Exception {
    Field field = PlaybookInvocationExtension.class.getDeclaredField("METHOD_SCOPE_KEY");
    field.setAccessible(true);
    return (String) field.get(null);
  }

  private static final class ParameterHolder {
    @SuppressWarnings("unused")
    void accept(ActiveHttpPlaybook active) {}

    @SuppressWarnings("unused")
    void accept(String value) {}
  }

  private static final class StubActiveHttpPlaybook extends ActiveHttpPlaybook {
    StubActiveHttpPlaybook() {
      super(new Pointer(1));
    }
  }

  private static final class StubNonHttpActivePlaybook extends ActivePlaybook {
    StubNonHttpActivePlaybook() {
      super(new Pointer(2));
    }
  }

  private static final class StubQuietActivePlaybook extends ActivePlaybook {
    StubQuietActivePlaybook() {
      super(new Pointer(3));
      noteBodyFailure();
    }
  }

  private static final class MapExtensionStore implements ExtensionContext.Store {
    private final Map<Object, Object> values = new HashMap<>();

    @Override
    public Object get(Object key) {
      return values.get(key);
    }

    @Override
    public <V> V get(Object key, Class<V> type) {
      return type.cast(values.get(key));
    }

    @Override
    public void put(Object key, Object value) {
      values.put(key, value);
    }

    @Override
    public Object remove(Object key) {
      return values.remove(key);
    }

    @Override
    public <V> V remove(Object key, Class<V> type) {
      return type.cast(values.remove(key));
    }

    @Override
    public <K, V> Object getOrComputeIfAbsent(
        K key, java.util.function.Function<K, V> defaultCreator) {
      @SuppressWarnings("unchecked")
      V existing = (V) values.get(key);
      if (existing != null) {
        return existing;
      }
      V created = defaultCreator.apply(key);
      values.put(key, created);
      return created;
    }

    @Override
    public <K, V> V getOrComputeIfAbsent(
        K key, java.util.function.Function<K, V> defaultCreator, Class<V> requiredType) {
      V existing = requiredType.cast(values.get(key));
      if (existing != null) {
        return existing;
      }
      V created = defaultCreator.apply(key);
      values.put(key, created);
      return created;
    }
  }

  private static final class MinimalExtensionContext implements ExtensionContext {
    private final ExtensionContext.Store store;
    private final Optional<java.lang.reflect.Method> testMethod;
    private final Optional<ExtensionContext> parent;
    private final Class<?> testClass;

    MinimalExtensionContext(
        ExtensionContext.Store store,
        Optional<java.lang.reflect.Method> testMethod,
        Optional<ExtensionContext> parent,
        Class<?> testClass) {
      this.store = store;
      this.testMethod = testMethod;
      this.parent = parent;
      this.testClass = testClass;
    }

    @Override
    public Optional<ExtensionContext> getParent() {
      return parent;
    }

    @Override
    public ExtensionContext getRoot() {
      return parent.map(ExtensionContext::getRoot).orElse(this);
    }

    @Override
    public String getUniqueId() {
      return "unit-test";
    }

    @Override
    public String getDisplayName() {
      return "unit-test";
    }

    @Override
    public Set<String> getTags() {
      return Set.of();
    }

    @Override
    public ExecutionMode getExecutionMode() {
      return ExecutionMode.SAME_THREAD;
    }

    @Override
    public Optional<Throwable> getExecutionException() {
      return Optional.empty();
    }

    @Override
    public Optional<org.junit.jupiter.api.TestInstance.Lifecycle> getTestInstanceLifecycle() {
      return Optional.empty();
    }

    @Override
    public Optional<TestInstances> getTestInstances() {
      return Optional.empty();
    }

    @Override
    public Optional<Class<?>> getTestClass() {
      return Optional.of(testClass);
    }

    @Override
    public Optional<java.lang.reflect.Method> getTestMethod() {
      return testMethod;
    }

    @Override
    public Optional<Object> getTestInstance() {
      return Optional.empty();
    }

    @Override
    public Optional<java.lang.reflect.AnnotatedElement> getElement() {
      return Optional.empty();
    }

    @Override
    public Class<?> getRequiredTestClass() {
      return testClass;
    }

    @Override
    public java.lang.reflect.Method getRequiredTestMethod() {
      return testMethod.orElseThrow();
    }

    @Override
    public Object getRequiredTestInstance() {
      throw new IllegalStateException("no test instance");
    }

    @Override
    public TestInstances getRequiredTestInstances() {
      throw new IllegalStateException("no test instances");
    }

    @Override
    public Optional<String> getConfigurationParameter(String key) {
      return Optional.empty();
    }

    @Override
    public <T> Optional<T> getConfigurationParameter(
        String key, java.util.function.Function<String, T> transformer) {
      return Optional.empty();
    }

    @Override
    public void publishReportEntry(Map<String, String> map) {}

    @Override
    public ExtensionContext.Store getStore(ExtensionContext.Namespace namespace) {
      return store;
    }

    @Override
    public ExecutableInvoker getExecutableInvoker() {
      throw new UnsupportedOperationException("not used in unit test");
    }
  }
}
