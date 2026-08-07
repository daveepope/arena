package arena.junit;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertSame;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import arena.junit.ffi.ArenaLogLevel;
import arena.junit.match.ArenaMatchPiece;
import arena.junit.match.Match;
import arena.junit.match.MatchBuilder;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.node.ObjectNode;
import java.lang.annotation.Annotation;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.lang.reflect.Parameter;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.util.Set;
import java.util.function.Function;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.TestInstance;
import org.junit.jupiter.api.extension.ExecutableInvoker;
import org.junit.jupiter.api.extension.ExtensionContext;
import org.junit.jupiter.api.extension.ParameterContext;
import org.junit.jupiter.api.extension.ParameterResolutionException;
import org.junit.jupiter.api.extension.TestInstances;
import org.junit.jupiter.api.parallel.ExecutionMode;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

final class ArenaExtensionUnitTest {

  private static final ObjectMapper MAPPER = new ObjectMapper();

  static final class StubMatchPiece implements ArenaMatchPiece {
    @Override
    public ObjectNode forFfi() {
      return MAPPER.createObjectNode();
    }
  }

  static final class IdentifiedMatchPiece implements ArenaMatchPiece {
    private final String identifier;

    IdentifiedMatchPiece(String identifier) {
      this.identifier = identifier;
    }

    @Override
    public ObjectNode forFfi() {
      return MAPPER.createObjectNode().put("identifier", identifier);
    }
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
