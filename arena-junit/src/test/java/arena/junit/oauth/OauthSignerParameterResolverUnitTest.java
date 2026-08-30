package arena.junit.oauth;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import arena.junit.ArenaDependency;
import com.fasterxml.jackson.databind.node.JsonNodeFactory;
import java.lang.reflect.Method;
import java.util.Optional;
import java.util.Set;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.extension.ExecutableInvoker;
import org.junit.jupiter.api.extension.ExtensionContext;
import org.junit.jupiter.api.extension.ParameterContext;
import org.junit.jupiter.api.extension.TestInstances;
import org.junit.jupiter.api.parallel.ExecutionMode;

final class OauthSignerParameterResolverUnitTest {

  private final OauthSignerParameterResolver resolver = new OauthSignerParameterResolver();

  @Test
  void supportsParameter_oauthSignerType_returnsTrue() throws Exception {
    ParameterContext context = parameterContext(ParameterHolder.class, "acceptSigner");
    assertTrue(resolver.supportsParameter(context, testClassContext(Object.class)));
  }

  @Test
  void supportsParameter_unrelatedType_returnsFalse() throws Exception {
    ParameterContext context = parameterContext(ParameterHolder.class, "acceptString");
    assertFalse(resolver.supportsParameter(context, testClassContext(Object.class)));
  }

  @Test
  void resolveParameter_noArenaAnnotationOnTestClass_throwsIllegalStateException() throws Exception {
    ParameterContext context = parameterContext(ParameterHolder.class, "acceptSigner");
    ExtensionContext extensionContext = testClassContext(NoArenaAnnotationHost.class);

    IllegalStateException error =
        assertThrows(IllegalStateException.class, () -> resolver.resolveParameter(context, extensionContext));

    assertTrue(error.getMessage().contains("@Arena"));
  }

  @Test
  void resolveParameter_noStaticOauthDependencyField_throwsIllegalStateException() throws Exception {
    ParameterContext context = parameterContext(ParameterHolder.class, "acceptSigner");
    ExtensionContext extensionContext = testClassContext(NoOauthDependencyFieldHost.class);

    IllegalStateException error =
        assertThrows(IllegalStateException.class, () -> resolver.resolveParameter(context, extensionContext));

    assertTrue(error.getMessage().contains("found 0"));
  }

  @Test
  void resolveParameter_multipleStaticOauthDependencyFields_throwsIllegalStateException() throws Exception {
    ParameterContext context = parameterContext(ParameterHolder.class, "acceptSigner");
    ExtensionContext extensionContext = testClassContext(MultipleOauthDependencyFieldsHost.class);

    IllegalStateException error =
        assertThrows(IllegalStateException.class, () -> resolver.resolveParameter(context, extensionContext));

    assertTrue(error.getMessage().contains("found 2"));
  }

  private static ParameterContext parameterContext(Class<?> holder, String methodName) throws Exception {
    Method method = holder.getDeclaredMethod(methodName, methodParameterType(holder, methodName));
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
      public <A extends java.lang.annotation.Annotation> java.util.List<A> findRepeatableAnnotations(
          Class<A> annotationType) {
        return java.util.List.of();
      }
    };
  }

  private static Class<?> methodParameterType(Class<?> holder, String methodName) {
    return "acceptSigner".equals(methodName) ? OauthSigner.class : String.class;
  }

  private static ExtensionContext testClassContext(Class<?> testClass) {
    MapExtensionStore store = new MapExtensionStore();
    return new ExtensionContext() {
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
        throw new IllegalStateException("no test method");
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
      public void publishReportEntry(java.util.Map<String, String> map) {}

      @Override
      public ExtensionContext.Store getStore(ExtensionContext.Namespace namespace) {
        return store;
      }

      @Override
      public ExecutableInvoker getExecutableInvoker() {
        throw new UnsupportedOperationException("not used in unit test");
      }
    };
  }

  private static final class ParameterHolder {
    @SuppressWarnings("unused")
    void acceptSigner(OauthSigner signer) {}

    @SuppressWarnings("unused")
    void acceptString(String value) {}
  }

  private static final class NoArenaAnnotationHost {}

  @arena.junit.Arena(NoOauthDependencyFieldHost.Fixture.class)
  private static final class NoOauthDependencyFieldHost {
    static final class Fixture {}
  }

  @arena.junit.Arena(MultipleOauthDependencyFieldsHost.Fixture.class)
  private static final class MultipleOauthDependencyFieldsHost {
    static final class Fixture {
      @ArenaDependency
      static final OauthDependency OAUTH_ONE =
          new OauthDependency(JsonNodeFactory.instance.objectNode().put("identifier", "oauth-one"));

      @ArenaDependency
      static final OauthDependency OAUTH_TWO =
          new OauthDependency(JsonNodeFactory.instance.objectNode().put("identifier", "oauth-two"));
    }
  }

  private static final class MapExtensionStore implements ExtensionContext.Store {
    private final java.util.Map<Object, Object> values = new java.util.HashMap<>();

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
    public <K, V> Object getOrComputeIfAbsent(K key, java.util.function.Function<K, V> defaultCreator) {
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
}
