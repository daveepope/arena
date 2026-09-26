package arena.junit.oauth;

import org.junit.jupiter.api.extension.ExtensionContext;
import org.junit.jupiter.api.extension.ParameterContext;
import org.junit.jupiter.api.extension.ParameterResolver;

public final class OauthSignerParameterResolver implements ParameterResolver {

  @Override
  public boolean supportsParameter(ParameterContext parameterContext, ExtensionContext extensionContext) {
    return OauthSigner.class.isAssignableFrom(parameterContext.getParameter().getType());
  }

  @Override
  public Object resolveParameter(ParameterContext parameterContext, ExtensionContext extensionContext) {
    Class<?> testClass = extensionContext.getRequiredTestClass();
    return OauthSigner.forFixture(fixtureClassFor(testClass));
  }

  private static Class<?> fixtureClassFor(Class<?> testClass) {
    arena.junit.Arena annotation = testClass.getAnnotation(arena.junit.Arena.class);
    if (annotation == null) {
      throw new IllegalStateException(
          "OauthSigner injection requires @Arena(FixtureClass.class) on the test class");
    }
    return annotation.value();
  }
}
