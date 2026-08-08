package arena.junit;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertSame;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import arena.examples.testruntime.EphemeralTestRuntime;
import arena.junit.ffi.ArenaLogLevel;
import arena.junit.ffi.ArenaLogbackFlush;
import arena.junit.oauth.OauthDependency;
import arena.junit.oauth.OauthDependencyBuilder;
import arena.junit.oauth.OauthLoopbackTls;
import ch.qos.logback.classic.LoggerContext;
import ch.qos.logback.classic.spi.ILoggingEvent;
import ch.qos.logback.core.read.ListAppender;
import java.lang.reflect.Method;
import java.util.List;
import java.util.Map;
import java.util.Objects;
import java.util.Optional;
import java.util.Set;
import java.util.function.Function;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.TestInstance;
import org.junit.jupiter.api.extension.ExecutableInvoker;
import org.junit.jupiter.api.extension.ExtensionContext;
import org.junit.jupiter.api.extension.TestInstances;
import org.junit.jupiter.api.parallel.ExecutionMode;
import org.junit.platform.suite.api.SelectClasses;
import org.junit.platform.suite.api.Suite;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

final class ArenaExtensionLifecycleComponentTest {

  private static final EphemeralTestRuntime RT = EphemeralTestRuntime.get();

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

  static final class SuiteDefs {
    @ArenaDependency
    static final OauthDependency oauth =
        buildOauth("arena-extension-suite-defs-oauth", EphemeralTestRuntime.ephemeralTcpPort());
  }

  @Arena(SuiteDefs.class)
  static final class SuiteMemberA {}

  @Arena(SuiteDefs.class)
  static final class SuiteMemberB {}

  static final class EmptySuiteDefs {}

  @Arena(EmptySuiteDefs.class)
  static final class SuiteMemberMissingFields {}

  static final class UnrelatedSelectedClass {}

  @Suite
  @SelectClasses(UnrelatedSelectedClass.class)
  static final class MismatchedSuiteDefs {
    @ArenaDependency
    static final OauthDependency oauth =
        buildOauth("arena-extension-mismatched-suite-oauth", EphemeralTestRuntime.ephemeralTcpPort());
  }

  @Arena(MismatchedSuiteDefs.class)
  static final class MismatchedMemberA {}

  @Arena(MismatchedSuiteDefs.class)
  static final class MismatchedMemberB {}

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

  static final class DependencyLogsEnabledTopology {
    static final String OAUTH_IDENTIFIER = "arena-extension-dependency-logs-enabled-oauth";

    @ArenaDependency(logs = true)
    static final OauthDependency oauth =
        buildOauth(OAUTH_IDENTIFIER, EphemeralTestRuntime.ephemeralTcpPort());

    @ArenaLogger(level = ArenaLogLevel.DEBUG)
    static final Logger LOG = LoggerFactory.getLogger(DependencyLogsEnabledTopology.class);
  }

  static final class DependencyLogsDisabledTopology {
    static final String OAUTH_IDENTIFIER = "arena-extension-dependency-logs-disabled-oauth";

    @ArenaDependency
    static final OauthDependency oauth =
        buildOauth(OAUTH_IDENTIFIER, EphemeralTestRuntime.ephemeralTcpPort());

    @ArenaLogger(level = ArenaLogLevel.DEBUG)
    static final Logger LOG = LoggerFactory.getLogger(DependencyLogsDisabledTopology.class);
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
  void beforeAll_unrelatedClassesReferencingSameArenaViaExplicitValue_opensOnceAndReuses() {
    ArenaExtension extension = new ArenaExtension();

    extension.beforeAll(contextFor(SuiteMemberA.class));
    OpenArena first = ArenaExtension.openArenaFor(SuiteMemberA.class);
    assertNotNull(first);
    assertTrue(first.handle() != null);

    extension.beforeAll(contextFor(SuiteMemberB.class));
    OpenArena second = ArenaExtension.openArenaFor(SuiteMemberB.class);
    assertSame(first, second);

    extension.afterAll(contextFor(SuiteMemberA.class));
    assertNotNull(first.handle());

    extension.afterAll(contextFor(SuiteMemberB.class));
    assertNull(first.handle());
  }

  @Test
  void beforeAll_suiteRootSelectClassesDoesNotNameExplicitMembers_fallsBackToRefCountingInsteadOfClosingEarly() {
    ArenaExtension extension = new ArenaExtension();

    extension.beforeAll(contextFor(MismatchedMemberA.class));
    extension.beforeAll(contextFor(MismatchedMemberB.class));
    OpenArena opened = ArenaExtension.openArenaFor(MismatchedMemberA.class);
    assertNotNull(opened.handle());

    extension.afterAll(contextFor(MismatchedMemberA.class));
    assertNotNull(
        opened.handle(),
        "arena must stay open for MismatchedMemberB even though @SelectClasses on "
            + "MismatchedSuiteDefs does not name either explicit member");

    extension.afterAll(contextFor(MismatchedMemberB.class));
    assertNull(opened.handle());
  }

  @Test
  void beforeAll_explicitValueReferencesClassWithNoArenaFields_throwsIllegalStateException() {
    ArenaExtension extension = new ArenaExtension();
    IllegalStateException error =
        assertThrows(
            IllegalStateException.class,
            () -> extension.beforeAll(contextFor(SuiteMemberMissingFields.class)));
    assertTrue(error.getMessage().contains(EmptySuiteDefs.class.getName()));
    assertTrue(error.getMessage().contains(SuiteMemberMissingFields.class.getName()));
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
  void beforeAll_dependencyLogsEnabled_forwardsDependencyTaggedDebugLog() {
    ArenaExtension extension = new ArenaExtension();
    ListAppender<ILoggingEvent> capture = attachCapture(DependencyLogsEnabledTopology.class);
    try {
      extension.beforeAll(contextFor(DependencyLogsEnabledTopology.class));
      ArenaLogbackFlush.flushIfPresent();
      assertTrue(
          capture.list.stream()
              .anyMatch(
                  ev ->
                      safeMessage(ev).contains(DependencyLogsEnabledTopology.OAUTH_IDENTIFIER)
                          && safeMessage(ev).contains("starting")),
          capture.list::toString);
    } finally {
      extension.afterAll(contextFor(DependencyLogsEnabledTopology.class));
      detachCapture(DependencyLogsEnabledTopology.class, capture);
    }
  }

  @Test
  void beforeAll_dependencyLogsDisabled_dependencyTaggedDebugLogNotForwarded() {
    ArenaExtension extension = new ArenaExtension();
    ListAppender<ILoggingEvent> capture = attachCapture(DependencyLogsDisabledTopology.class);
    try {
      extension.beforeAll(contextFor(DependencyLogsDisabledTopology.class));
      ArenaLogbackFlush.flushIfPresent();
      assertTrue(
          capture.list.stream()
              .noneMatch(
                  ev ->
                      safeMessage(ev).contains(DependencyLogsDisabledTopology.OAUTH_IDENTIFIER)
                          && safeMessage(ev).contains("starting")),
          capture.list::toString);
    } finally {
      extension.afterAll(contextFor(DependencyLogsDisabledTopology.class));
      detachCapture(DependencyLogsDisabledTopology.class, capture);
    }
  }

  private static ExtensionContext contextFor(Class<?> testClass) {
    return new MinimalExtensionContext(testClass);
  }

  private static ListAppender<ILoggingEvent> attachCapture(Class<?> loggerOwner) {
    ListAppender<ILoggingEvent> capture = new ListAppender<>();
    capture.setContext(backlogContext());
    capture.setName("capture-" + loggerOwner.getSimpleName() + "-" + Objects.hash(loggerOwner));
    capture.start();
    backlogLogger(loggerOwner).addAppender(capture);
    return capture;
  }

  private static void detachCapture(Class<?> loggerOwner, ListAppender<ILoggingEvent> capture) {
    capture.stop();
    backlogLogger(loggerOwner).detachAppender(capture);
  }

  private static ch.qos.logback.classic.Logger backlogLogger(Class<?> loggerOwner) {
    Logger facade = LoggerFactory.getLogger(loggerOwner);
    if (!(facade instanceof ch.qos.logback.classic.Logger)) {
      throw new AssertionError("org.slf4j.Logger must bridge to Logback classic Logger here");
    }
    return (ch.qos.logback.classic.Logger) facade;
  }

  private static LoggerContext backlogContext() {
    if (!(LoggerFactory.getILoggerFactory() instanceof LoggerContext)) {
      throw new AssertionError("Logback LoggerContext expected on test classpath");
    }
    return (LoggerContext) LoggerFactory.getILoggerFactory();
  }

  private static String safeMessage(ILoggingEvent ev) {
    String formatted = ev.getFormattedMessage();
    if (formatted != null && !formatted.isEmpty()) {
      return formatted;
    }
    String raw = ev.getMessage();
    return raw != null ? raw : "";
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
