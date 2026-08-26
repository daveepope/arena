package arena.junit;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertSame;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import arena.junit.ffi.ArenaLogLevel;
import arena.junit.match.ArenaRunnableComponent;
import arena.junit.match.ArenaRunnableDependency;
import arena.junit.match.Match;
import arena.junit.match.MatchBuilder;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.node.ObjectNode;
import java.lang.annotation.Annotation;
import java.lang.reflect.Constructor;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.lang.reflect.Parameter;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.util.Set;
import java.util.concurrent.ConcurrentHashMap;
import java.util.function.Function;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.TestInstance;
import org.junit.jupiter.api.extension.ExecutableInvoker;
import org.junit.jupiter.api.extension.ExtensionContext;
import org.junit.jupiter.api.extension.ParameterContext;
import org.junit.jupiter.api.extension.ParameterResolutionException;
import org.junit.jupiter.api.extension.TestInstances;
import org.junit.jupiter.api.parallel.ExecutionMode;
import org.junit.platform.suite.api.SelectClasses;
import org.junit.platform.suite.api.Suite;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

final class ArenaExtensionUnitTest {

  private static final ObjectMapper MAPPER = new ObjectMapper();

  static final class StubMatchPiece implements ArenaRunnableDependency, ArenaRunnableComponent {
    @Override
    public ObjectNode forFfi() {
      return MAPPER.createObjectNode();
    }
  }

  static final class IdentifiedMatchPiece implements ArenaRunnableDependency, ArenaRunnableComponent {
    private final String identifier;

    IdentifiedMatchPiece(String identifier) {
      this.identifier = identifier;
    }

    @Override
    public ObjectNode forFfi() {
      return MAPPER.createObjectNode().put("identifier", identifier);
    }
  }

  static final class CountingFailureMatchPiece implements ArenaRunnableDependency {
    static int forFfiCalls;

    @Override
    public ObjectNode forFfi() {
      forFfiCalls++;
      return MAPPER.createObjectNode();
    }
  }

  static final class BeforeAllFailureCachingTopology {
    @ArenaDependency static final CountingFailureMatchPiece dependency =
        new CountingFailureMatchPiece();
  }

  static final class NoArenaFieldsTopology {}

  static final class NonStaticDependencyFieldTopology {
    @ArenaDependency final StubMatchPiece dependency = new StubMatchPiece();
  }

  static final class TwoDependenciesSameTypeTopology {
    @ArenaDependency static final StubMatchPiece first = new StubMatchPiece();
    @ArenaDependency static final StubMatchPiece second = new StubMatchPiece();
  }

  static final class DuplicateLoggerFieldTopology {
    @ArenaDependency static final StubMatchPiece dependency = new StubMatchPiece();

    @ArenaLogger
    static final Logger FIRST = LoggerFactory.getLogger(DuplicateLoggerFieldTopology.class);

    @ArenaLogger
    static final Logger SECOND = LoggerFactory.getLogger(DuplicateLoggerFieldTopology.class);
  }

  static final class DependencyLogsTrueTopology {
    static final String IDENTIFIER = "dependency-logs-true-topology";

    @ArenaDependency(logs = true)
    static final IdentifiedMatchPiece dependency = new IdentifiedMatchPiece(IDENTIFIER);
  }

  static final class DependencyLogsFalseDefaultTopology {
    @ArenaDependency
    static final IdentifiedMatchPiece dependency =
        new IdentifiedMatchPiece("dependency-logs-false-default-topology");
  }

  static final class ComponentLogsTrueTopology {
    static final String IDENTIFIER = "component-logs-true-topology";

    @ArenaComponent(logs = true)
    static final IdentifiedMatchPiece component = new IdentifiedMatchPiece(IDENTIFIER);
  }

  static final class ComponentLogsFalseDefaultTopology {
    @ArenaComponent
    static final IdentifiedMatchPiece component =
        new IdentifiedMatchPiece("component-logs-false-default-topology");
  }

  static final class LoggerFieldTopology {
    @ArenaDependency static final StubMatchPiece dependency = new StubMatchPiece();

    @ArenaLogger(level = ArenaLogLevel.DEBUG)
    static final Logger LOG = LoggerFactory.getLogger(LoggerFieldTopology.class);
  }

  static final class NoLoggerFieldTopology {
    @ArenaDependency static final StubMatchPiece dependency = new StubMatchPiece();
  }

  @Arena
  static final class SelfReferencingArenaTopology {
    @ArenaDependency static final StubMatchPiece dependency = new StubMatchPiece();
  }

  static final class ExplicitRootDependencyTopology {
    @ArenaDependency static final StubMatchPiece dependency = new StubMatchPiece();
  }

  @Arena(ExplicitRootDependencyTopology.class)
  static final class ExplicitRootConsumerTopology {}

  @Arena(ExplicitRootDependencyTopology.class)
  abstract static class ExplicitRootAnnotatedBase {}

  static final class ExplicitRootInheritingConsumerTopology extends ExplicitRootAnnotatedBase {}

  static final class EmptyExplicitTargetTopology {}

  @Arena(EmptyExplicitTargetTopology.class)
  static final class ExplicitTargetMissingFieldsConsumerTopology {}

  @Arena(SelectClassesRootTopology.class)
  static final class SelectClassesMatchingMemberA {}

  @Arena(SelectClassesRootTopology.class)
  static final class SelectClassesMatchingMemberB {}

  static final class UnrelatedSelectClassesMember {}

  @Suite
  @SelectClasses({SelectClassesMatchingMemberA.class, SelectClassesMatchingMemberB.class})
  static final class SelectClassesRootTopology {
    @ArenaDependency static final StubMatchPiece dependency = new StubMatchPiece();
  }

  @Suite
  @SelectClasses(UnrelatedSelectClassesMember.class)
  static final class SelectClassesRootWithNoMatchingMembersTopology {
    @ArenaDependency static final StubMatchPiece dependency = new StubMatchPiece();
  }

  static final class BeforeAllSuiteMemberTopology {
    @ArenaDependency static final StubMatchPiece dependency = new StubMatchPiece();
  }

  static final class BeforeAllRefCountedTopology {
    @ArenaDependency static final StubMatchPiece dependency = new StubMatchPiece();
  }

  static final class AfterAllSuiteMemberCompleteTopology {
    @ArenaDependency static final StubMatchPiece dependency = new StubMatchPiece();
  }

  static final class AfterAllSuiteMemberPendingTopology {
    @ArenaDependency static final StubMatchPiece dependency = new StubMatchPiece();
  }

  static final class AfterAllRefCountedTopology {
    @ArenaDependency static final StubMatchPiece dependency = new StubMatchPiece();
  }

  static final class AfterAllFailedCacheEntryTopology {
    @ArenaDependency static final StubMatchPiece dependency = new StubMatchPiece();
  }

  @Test
  void beforeAll_noArenaAnnotatedFields_throwsIllegalStateException() {
    ArenaExtension extension = new ArenaExtension();
    IllegalStateException error =
        assertThrows(
            IllegalStateException.class,
            () -> extension.beforeAll(contextFor(NoArenaFieldsTopology.class)));
    assertTrue(error.getMessage().contains("@ArenaDependency"));
  }

  @Test
  void beforeAll_nonStaticDependencyField_throwsIllegalStateException() {
    ArenaExtension extension = new ArenaExtension();
    IllegalStateException error =
        assertThrows(
            IllegalStateException.class,
            () -> extension.beforeAll(contextFor(NonStaticDependencyFieldTopology.class)));
    assertTrue(error.getMessage().contains("must be static"));
  }

  @Test
  void beforeAll_duplicateLoggerFields_throwsIllegalStateException() {
    ArenaExtension extension = new ArenaExtension();
    IllegalStateException error =
        assertThrows(
            IllegalStateException.class,
            () -> extension.beforeAll(contextFor(DuplicateLoggerFieldTopology.class)));
    assertTrue(error.getMessage().contains("@ArenaLogger"));
  }

  @Test
  void supportsParameter_ambiguousFieldType_throwsParameterResolutionException() {
    ArenaExtension extension = new ArenaExtension();
    ExtensionContext context = contextFor(TwoDependenciesSameTypeTopology.class);
    ParameterContext parameterContext = parameterContextFor(StubMatchPiece.class);
    assertThrows(
        ParameterResolutionException.class,
        () -> extension.supportsParameter(parameterContext, context));
  }

  @Test
  void buildMatchBuilder_dependencyLogsTrue_addsIdentifierToDependencyIds() throws Exception {
    Object logIdentifiers = buildLogIdentifiers(DependencyLogsTrueTopology.class);
    assertEquals(List.of(DependencyLogsTrueTopology.IDENTIFIER), dependencyIds(logIdentifiers));
    assertEquals(List.of(), componentIds(logIdentifiers));
  }

  @Test
  void buildMatchBuilder_dependencyLogsFalseDefault_dependencyIdsEmpty() throws Exception {
    Object logIdentifiers = buildLogIdentifiers(DependencyLogsFalseDefaultTopology.class);
    assertEquals(List.of(), dependencyIds(logIdentifiers));
  }

  @Test
  void buildMatchBuilder_componentLogsTrue_addsIdentifierToComponentIds() throws Exception {
    Object logIdentifiers = buildLogIdentifiers(ComponentLogsTrueTopology.class);
    assertEquals(List.of(ComponentLogsTrueTopology.IDENTIFIER), componentIds(logIdentifiers));
    assertEquals(List.of(), dependencyIds(logIdentifiers));
  }

  @Test
  void buildMatchBuilder_componentLogsFalseDefault_componentIdsEmpty() throws Exception {
    Object logIdentifiers = buildLogIdentifiers(ComponentLogsFalseDefaultTopology.class);
    assertEquals(List.of(), componentIds(logIdentifiers));
  }

  @Test
  void closedArenaFor_loggerFieldPresent_usesFieldLevelAndLogger() throws Exception {
    ClosedArena closedArena = buildClosedArena(LoggerFieldTopology.class);
    assertEquals(ArenaLogLevel.DEBUG, readField(closedArena, "logLevel"));
    assertSame(
        LoggerFactory.getLogger(LoggerFieldTopology.class), readField(closedArena, "slf4jLogger"));
  }

  @Test
  void closedArenaFor_noLoggerField_defaultsToInfoLevelAndNullLogger() throws Exception {
    ClosedArena closedArena = buildClosedArena(NoLoggerFieldTopology.class);
    assertEquals(ArenaLogLevel.INFO, readField(closedArena, "logLevel"));
    assertNull(readField(closedArena, "slf4jLogger"));
  }

  @Test
  void explicitRoot_noArenaAnnotation_returnsNull() throws Exception {
    assertNull(invokeExplicitRoot(NoArenaFieldsTopology.class));
  }

  @Test
  void explicitRoot_arenaAnnotationDefaultValue_returnsNull() throws Exception {
    assertNull(invokeExplicitRoot(SelfReferencingArenaTopology.class));
  }

  @Test
  void explicitRoot_arenaAnnotationWithExplicitValue_returnsAnnotatedClass() throws Exception {
    assertEquals(
        ExplicitRootDependencyTopology.class, invokeExplicitRoot(ExplicitRootConsumerTopology.class));
  }

  @Test
  void explicitRoot_annotationDeclaredOnSuperclass_returnsAnnotatedClass() throws Exception {
    assertEquals(
        ExplicitRootDependencyTopology.class,
        invokeExplicitRoot(ExplicitRootInheritingConsumerTopology.class));
  }

  @Test
  void topologyRoot_explicitValuePresent_resolvesToExplicitTarget() throws Exception {
    assertEquals(
        ExplicitRootDependencyTopology.class, invokeTopologyRoot(ExplicitRootConsumerTopology.class));
  }

  @Test
  void beforeAll_explicitRootReferencesClassWithNoArenaFields_messageNamesBothClasses() {
    ArenaExtension extension = new ArenaExtension();
    IllegalStateException error =
        assertThrows(
            IllegalStateException.class,
            () -> extension.beforeAll(contextFor(ExplicitTargetMissingFieldsConsumerTopology.class)));
    assertTrue(error.getMessage().contains(EmptyExplicitTargetTopology.class.getName()));
    assertTrue(
        error
            .getMessage()
            .contains(
                "referenced by @Arena("
                    + EmptyExplicitTargetTopology.class.getSimpleName()
                    + ".class) on "
                    + ExplicitTargetMissingFieldsConsumerTopology.class.getName()));
  }

  @Test
  void selectClassesValue_noSelectClassesAnnotation_returnsNull() throws Exception {
    assertNull(invokeSelectClassesValue(NoLoggerFieldTopology.class));
  }

  @Test
  void selectClassesValue_selectClassesAnnotationPresent_returnsDeclaredClasses() throws Exception {
    Class<?>[] value = invokeSelectClassesValue(SelectClassesRootTopology.class);
    assertNotNull(value);
    assertEquals(2, value.length);
  }

  @Test
  void expectedSuiteMembers_noSelectClassesAnnotation_returnsNull() throws Exception {
    assertNull(invokeExpectedSuiteMembers(NoLoggerFieldTopology.class));
  }

  @Test
  void expectedSuiteMembers_selectedClassesReferenceRootExplicitly_returnsMemberCount() throws Exception {
    assertEquals(2, invokeExpectedSuiteMembers(SelectClassesRootTopology.class));
  }

  @Test
  void expectedSuiteMembers_selectedClassesDoNotReferenceRoot_returnsNull() throws Exception {
    assertNull(invokeExpectedSuiteMembers(SelectClassesRootWithNoMatchingMembersTopology.class));
  }

  @Test
  void warnIfExplicitRootMissingSelectClasses_noExplicitRoot_doesNotWarn() throws Exception {
    invokeWarnIfExplicitRootMissingSelectClasses(NoArenaFieldsTopology.class, NoArenaFieldsTopology.class);
    assertFalse(warnedMissingSelectClasses().contains(NoArenaFieldsTopology.class));
  }

  @Test
  void warnIfExplicitRootMissingSelectClasses_rootHasSelectClasses_doesNotWarn() throws Exception {
    invokeWarnIfExplicitRootMissingSelectClasses(
        SelectClassesMatchingMemberA.class, SelectClassesRootTopology.class);
    assertFalse(warnedMissingSelectClasses().contains(SelectClassesRootTopology.class));
  }

  @Test
  void warnIfExplicitRootMissingSelectClasses_explicitRootWithoutSelectClasses_addsRootToWarnedSet()
      throws Exception {
    invokeWarnIfExplicitRootMissingSelectClasses(
        ExplicitRootConsumerTopology.class, ExplicitRootDependencyTopology.class);
    assertTrue(warnedMissingSelectClasses().contains(ExplicitRootDependencyTopology.class));

    // repeated call for the same root must not error even though the warned set no longer accepts it
    invokeWarnIfExplicitRootMissingSelectClasses(
        ExplicitRootConsumerTopology.class, ExplicitRootDependencyTopology.class);
    assertTrue(warnedMissingSelectClasses().contains(ExplicitRootDependencyTopology.class));
  }

  @Test
  void beforeAll_cachedArenaWithExpectedSuiteMembers_doesNotIncrementRefs() throws Exception {
    Class<?> root = BeforeAllSuiteMemberTopology.class;
    Object cached = newCachedArena(new OpenArena(null, 0L, List.of()), 2);
    cache().put(root, cached);
    try {
      new ArenaExtension().beforeAll(contextFor(root));
      assertEquals(0, getIntField(cached, "refs"));
    } finally {
      cache().remove(root);
    }
  }

  @Test
  void beforeAll_cachedArenaWithoutExpectedSuiteMembers_incrementsRefs() throws Exception {
    Class<?> root = BeforeAllRefCountedTopology.class;
    Object cached = newCachedArena(new OpenArena(null, 0L, List.of()), null);
    cache().put(root, cached);
    try {
      new ArenaExtension().beforeAll(contextFor(root));
      assertEquals(1, getIntField(cached, "refs"));
    } finally {
      cache().remove(root);
    }
  }

  @Test
  void beforeAll_dependencyOpenFails_cachesFailureAndDoesNotReopenOnRepeatedCalls() throws Exception {
    Class<?> root = BeforeAllFailureCachingTopology.class;
    CountingFailureMatchPiece.forFfiCalls = 0;
    ArenaExtension extension = new ArenaExtension();
    try {
      Throwable first =
          assertThrows(IllegalStateException.class, () -> extension.beforeAll(contextFor(root)));
      Throwable second =
          assertThrows(IllegalStateException.class, () -> extension.beforeAll(contextFor(root)));
      assertSame(first, second);
      assertEquals(1, CountingFailureMatchPiece.forFfiCalls);
    } finally {
      cache().remove(root);
    }
  }

  @Test
  void afterAll_suiteCompletedCountReachesExpectedMembers_closesAndRemovesCacheEntry() throws Exception {
    Class<?> root = AfterAllSuiteMemberCompleteTopology.class;
    Object cached = newCachedArena(new OpenArena(null, 0L, List.of()), 2);
    setIntField(cached, "completed", 1);
    cache().put(root, cached);
    new ArenaExtension().afterAll(contextFor(root));
    assertFalse(cache().containsKey(root));
  }

  @Test
  void afterAll_suiteCompletedCountBelowExpectedMembers_keepsCacheEntryOpen() throws Exception {
    Class<?> root = AfterAllSuiteMemberPendingTopology.class;
    Object cached = newCachedArena(new OpenArena(null, 0L, List.of()), 2);
    cache().put(root, cached);
    try {
      new ArenaExtension().afterAll(contextFor(root));
      assertTrue(cache().containsKey(root));
      assertEquals(1, getIntField(cached, "completed"));
    } finally {
      cache().remove(root);
    }
  }

  @Test
  void afterAll_refCountFallbackReachesZero_closesAndRemovesCacheEntry() throws Exception {
    Class<?> root = AfterAllRefCountedTopology.class;
    Object cached = newCachedArena(new OpenArena(null, 0L, List.of()), null);
    setIntField(cached, "refs", 1);
    cache().put(root, cached);
    new ArenaExtension().afterAll(contextFor(root));
    assertFalse(cache().containsKey(root));
  }

  @Test
  void afterAll_cachedFailureRefCountReachesZero_removesCacheEntryWithoutClosing() throws Exception {
    Class<?> root = AfterAllFailedCacheEntryTopology.class;
    Object cached = newFailedCachedArena(new IllegalStateException("open failed"), null);
    setIntField(cached, "refs", 1);
    cache().put(root, cached);
    new ArenaExtension().afterAll(contextFor(root));
    assertFalse(cache().containsKey(root));
  }

  private static Class<?> invokeExplicitRoot(Class<?> testClass) throws Exception {
    Method m = ArenaExtension.class.getDeclaredMethod("explicitRoot", Class.class);
    m.setAccessible(true);
    return (Class<?>) m.invoke(null, testClass);
  }

  private static Class<?> invokeTopologyRoot(Class<?> testClass) throws Exception {
    Method m = ArenaExtension.class.getDeclaredMethod("topologyRoot", Class.class);
    m.setAccessible(true);
    return (Class<?>) m.invoke(null, testClass);
  }

  private static Class<?>[] invokeSelectClassesValue(Class<?> root) throws Exception {
    Method m = ArenaExtension.class.getDeclaredMethod("selectClassesValue", Class.class);
    m.setAccessible(true);
    return (Class<?>[]) m.invoke(null, root);
  }

  private static Integer invokeExpectedSuiteMembers(Class<?> root) throws Exception {
    Method m = ArenaExtension.class.getDeclaredMethod("expectedSuiteMembers", Class.class);
    m.setAccessible(true);
    return (Integer) m.invoke(null, root);
  }

  private static void invokeWarnIfExplicitRootMissingSelectClasses(Class<?> testClass, Class<?> root)
      throws Exception {
    Method m =
        ArenaExtension.class.getDeclaredMethod(
            "warnIfExplicitRootMissingSelectClasses", Class.class, Class.class);
    m.setAccessible(true);
    m.invoke(null, testClass, root);
  }

  @SuppressWarnings("unchecked")
  private static Set<Class<?>> warnedMissingSelectClasses() throws Exception {
    Field f = ArenaExtension.class.getDeclaredField("WARNED_MISSING_SELECT_CLASSES");
    f.setAccessible(true);
    return (Set<Class<?>>) f.get(null);
  }

  @SuppressWarnings("unchecked")
  private static ConcurrentHashMap<Class<?>, Object> cache() throws Exception {
    Field f = ArenaExtension.class.getDeclaredField("CACHE");
    f.setAccessible(true);
    return (ConcurrentHashMap<Class<?>, Object>) f.get(null);
  }

  private static Object newCachedArena(OpenArena openArena, Integer expectedSuiteMembers) throws Exception {
    Constructor<?> ctor = cachedArenaClass().getDeclaredConstructor(OpenArena.class, Integer.class);
    ctor.setAccessible(true);
    return ctor.newInstance(openArena, expectedSuiteMembers);
  }

  private static Object newFailedCachedArena(RuntimeException failure, Integer expectedSuiteMembers)
      throws Exception {
    Constructor<?> ctor =
        cachedArenaClass().getDeclaredConstructor(RuntimeException.class, Integer.class);
    ctor.setAccessible(true);
    return ctor.newInstance(failure, expectedSuiteMembers);
  }

  private static Class<?> cachedArenaClass() {
    for (Class<?> nested : ArenaExtension.class.getDeclaredClasses()) {
      if (nested.getSimpleName().equals("CachedArena")) {
        return nested;
      }
    }
    throw new IllegalStateException("ArenaExtension.CachedArena nested class not found");
  }

  private static void setIntField(Object instance, String name, int value) throws Exception {
    Field f = instance.getClass().getDeclaredField(name);
    f.setAccessible(true);
    f.setInt(instance, value);
  }

  private static int getIntField(Object instance, String name) throws Exception {
    Field f = instance.getClass().getDeclaredField(name);
    f.setAccessible(true);
    return f.getInt(instance);
  }

  private static Object buildLogIdentifiers(Class<?> root) throws Exception {
    Method buildMatchBuilder = ArenaExtension.class.getDeclaredMethod("buildMatchBuilder", Class.class);
    buildMatchBuilder.setAccessible(true);
    Object matchBuild = buildMatchBuilder.invoke(null, root);
    return invokeAccessor(matchBuild, "logIdentifiers");
  }

  private static ClosedArena buildClosedArena(Class<?> root) throws Exception {
    Method buildMatchBuilder = ArenaExtension.class.getDeclaredMethod("buildMatchBuilder", Class.class);
    buildMatchBuilder.setAccessible(true);
    Object matchBuild = buildMatchBuilder.invoke(null, root);
    MatchBuilder matchBuilder = (MatchBuilder) invokeAccessor(matchBuild, "matchBuilder");
    Object logIdentifiers = invokeAccessor(matchBuild, "logIdentifiers");
    Match match = matchBuilder.build();

    Method closedArenaFor =
        ArenaExtension.class.getDeclaredMethod(
            "closedArenaFor", Class.class, Match.class, logIdentifiers.getClass());
    closedArenaFor.setAccessible(true);
    return (ClosedArena) closedArenaFor.invoke(null, root, match, logIdentifiers);
  }

  @SuppressWarnings("unchecked")
  private static List<String> dependencyIds(Object logIdentifiers) throws Exception {
    return (List<String>) invokeAccessor(logIdentifiers, "dependencyIds");
  }

  @SuppressWarnings("unchecked")
  private static List<String> componentIds(Object logIdentifiers) throws Exception {
    return (List<String>) invokeAccessor(logIdentifiers, "componentIds");
  }

  private static Object invokeAccessor(Object recordInstance, String accessorName) throws Exception {
    Method accessor = recordInstance.getClass().getDeclaredMethod(accessorName);
    accessor.setAccessible(true);
    return accessor.invoke(recordInstance);
  }

  private static Object readField(Object instance, String fieldName) throws Exception {
    Field field = instance.getClass().getDeclaredField(fieldName);
    field.setAccessible(true);
    return field.get(instance);
  }

  private static ExtensionContext contextFor(Class<?> testClass) {
    return new MinimalExtensionContext(testClass);
  }

  private static ParameterContext parameterContextFor(Class<?> parameterType) {
    Method method;
    try {
      method = ParameterHolder.class.getDeclaredMethod("accept", parameterType);
    } catch (NoSuchMethodException e) {
      throw new IllegalStateException(e);
    }
    Method finalMethod = method;
    return new ParameterContext() {
      @Override
      public Parameter getParameter() {
        return finalMethod.getParameters()[0];
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
      public boolean isAnnotated(Class<? extends Annotation> annotationType) {
        return false;
      }

      @Override
      public <A extends Annotation> Optional<A> findAnnotation(Class<A> annotationType) {
        return Optional.empty();
      }

      @Override
      public <A extends Annotation> List<A> findRepeatableAnnotations(Class<A> annotationType) {
        return List.of();
      }
    };
  }

  private static final class ParameterHolder {
    @SuppressWarnings("unused")
    void accept(StubMatchPiece value) {}
  }

  private static final class MinimalExtensionContext implements ExtensionContext {
    private final Class<?> testClass;

    MinimalExtensionContext(Class<?> testClass) {
      this.testClass = testClass;
    }

    @Override
    public Optional<ExtensionContext> getParent() {
      return Optional.empty();
    }

    @Override
    public ExtensionContext getRoot() {
      return this;
    }

    @Override
    public String getUniqueId() {
      return "arena-extension-unit-test";
    }

    @Override
    public String getDisplayName() {
      return "arena-extension-unit-test";
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
    public Optional<TestInstance.Lifecycle> getTestInstanceLifecycle() {
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
    public Optional<Method> getTestMethod() {
      return Optional.empty();
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
    public Method getRequiredTestMethod() {
      throw new UnsupportedOperationException("not used in this test");
    }

    @Override
    public Object getRequiredTestInstance() {
      throw new UnsupportedOperationException("not used in this test");
    }

    @Override
    public TestInstances getRequiredTestInstances() {
      throw new UnsupportedOperationException("not used in this test");
    }

    @Override
    public Optional<String> getConfigurationParameter(String key) {
      return Optional.empty();
    }

    @Override
    public <T> Optional<T> getConfigurationParameter(String key, Function<String, T> transformer) {
      return Optional.empty();
    }

    @Override
    public void publishReportEntry(Map<String, String> map) {}

    @Override
    public ExtensionContext.Store getStore(ExtensionContext.Namespace namespace) {
      throw new UnsupportedOperationException("not used in this test");
    }

    @Override
    public ExecutableInvoker getExecutableInvoker() {
      throw new UnsupportedOperationException("not used in this test");
    }
  }
}
