package arena.junit;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertSame;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import arena.examples.testruntime.EphemeralTestRuntime;
import arena.junit.match.ArenaMatchPiece;
import arena.junit.oauth.OauthDependency;
import arena.junit.oauth.OauthDependencyBuilder;
import arena.junit.oauth.OauthLoopbackTls;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.node.ObjectNode;
import java.lang.annotation.Annotation;
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

final class ArenaExtensionLifecycleComponentTest {

  private static final EphemeralTestRuntime RT = EphemeralTestRuntime.get();
  private static final ObjectMapper MAPPER = new ObjectMapper();

  static final class StubMatchPiece implements ArenaMatchPiece {
    @Override
    public ObjectNode forFfi() {
      return MAPPER.createObjectNode();
    }
  }

  abstract static class SharedTopology {
    static int afterOpenCount;
    static OpenArena receivedOpenArena;

    @ArenaDependency
    static final OauthDependency oauth = buildOauth("arena-extension-lifecycle-oauth", RT.oauthPort);

    @ArenaLogger
    static final Logger LOG = LoggerFactory.getLogger(SharedTopology.class);

    @ArenaAfterOpen
    static void afterOpen(OpenArena arena) {
      afterOpenCount++;
      receivedOpenArena = arena;
    }
  }

  static final class FirstConsumer extends SharedTopology {}

  abstract static class MiddleLayer extends SharedTopology {}

  static final class SecondConsumer extends MiddleLayer {}

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

  static final class NonStaticAfterOpenTopology {
    @ArenaDependency
    static final OauthDependency oauth =
        buildOauth("arena-extension-nonstatic-oauth", EphemeralTestRuntime.ephemeralTcpPort());

    @ArenaAfterOpen
    void afterOpen() {}
  }

  static final class ThrowingAfterOpenTopology {
    @ArenaDependency
    static final OauthDependency oauth =
        buildOauth("arena-extension-throwing-oauth", EphemeralTestRuntime.ephemeralTcpPort());

    @ArenaAfterOpen
    static void afterOpen() {
      throw new RuntimeException("boom");
    }
  }

  static final class DuplicateAfterOpenTopology {
    @ArenaDependency
    static final OauthDependency oauth =
        buildOauth("arena-extension-duplicate-afteropen-oauth", EphemeralTestRuntime.ephemeralTcpPort());

    @ArenaAfterOpen
    static void first() {}

    @ArenaAfterOpen
    static void second() {}
  }

  private static OauthDependency buildOauth(String identifier, int port) {
    OauthLoopbackTls.PemPair pem = OauthLoopbackTls.oauthLoopbackTlsPemPair();
    return new OauthDependencyBuilder(identifier)
        .withPort(port)
        .withListenIp("0.0.0.0")
        .withServerTlsPem(pem.certificatePem(), pem.privateKeyPem())
        .withMetadataBaseUrl(RT.oauthIssuer)
        .build();
  }

  @Test
  void beforeAll_multipleClassesSharingTopologyAcrossInheritanceDepths_opensOnceAndReuses() {
    ArenaExtension extension = new ArenaExtension();

    extension.beforeAll(contextFor(FirstConsumer.class));
    OpenArena first = ArenaExtension.openArenaFor(FirstConsumer.class);
    assertNotNull(first);
    assertTrue(first.handle() != null);
    assertEquals(1, SharedTopology.afterOpenCount);
    assertSame(first, SharedTopology.receivedOpenArena);

    extension.beforeAll(contextFor(SecondConsumer.class));
    OpenArena second = ArenaExtension.openArenaFor(SecondConsumer.class);
    assertSame(first, second);
    assertEquals(1, SharedTopology.afterOpenCount);

    extension.afterAll(contextFor(FirstConsumer.class));
    assertNotNull(first.handle());

    extension.afterAll(contextFor(SecondConsumer.class));
    assertNull(first.handle());
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
  void beforeAll_nonStaticAfterOpenMethod_throwsIllegalStateExceptionAndAfterAllIsNoOp() {
    ArenaExtension extension = new ArenaExtension();

    IllegalStateException error =
        assertThrows(
            IllegalStateException.class,
            () -> extension.beforeAll(contextFor(NonStaticAfterOpenTopology.class)));
    assertTrue(error.getMessage().contains("must be static"));

    extension.afterAll(contextFor(NonStaticAfterOpenTopology.class));
  }

  @Test
  void beforeAll_afterOpenMethodThrows_wrapsInIllegalStateException() {
    ArenaExtension extension = new ArenaExtension();
    IllegalStateException error =
        assertThrows(
            IllegalStateException.class,
            () -> extension.beforeAll(contextFor(ThrowingAfterOpenTopology.class)));
    assertNotNull(error.getCause());
  }

  @Test
  void beforeAll_duplicateAfterOpenMethods_throwsIllegalStateException() {
    ArenaExtension extension = new ArenaExtension();
    assertThrows(
        IllegalStateException.class,
        () -> extension.beforeAll(contextFor(DuplicateAfterOpenTopology.class)));
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
      return "arena-extension-lifecycle-component-test";
    }

    @Override
    public String getDisplayName() {
      return "arena-extension-lifecycle-component-test";
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
