package arena.junit.oauth;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import arena.junit.ArenaDependency;
import com.fasterxml.jackson.databind.node.JsonNodeFactory;
import java.lang.reflect.Field;
import java.lang.reflect.InvocationTargetException;
import java.lang.reflect.Method;
import org.junit.jupiter.api.Test;

final class OauthSignerUnitTest {

  @Test
  void discoverOauthDependencyField_singleArenaDependencyField_returnsThatField() throws Exception {
    Field field = invokeDiscoverOauthDependencyField(SingleOauthDependencyFieldHost.Fixture.class);

    assertEquals("OAUTH", field.getName());
  }

  @Test
  void discoverOauthDependencyField_fieldDeclaredOnSuperclassFixture_returnsThatField() throws Exception {
    Field field = invokeDiscoverOauthDependencyField(InheritedOauthDependencyFieldHost.SubFixture.class);

    assertEquals("OAUTH", field.getName());
    assertEquals(BaseFixtureWithOauthDependency.class, field.getDeclaringClass());
  }

  @Test
  void discoverOauthDependencyField_noArenaDependencyField_throwsIllegalStateException() {
    IllegalStateException error =
        assertThrows(
            IllegalStateException.class,
            () -> invokeDiscoverOauthDependencyField(NoOauthDependencyFieldHost.Fixture.class));

    assertTrue(error.getMessage().contains("found 0"));
  }

  @Test
  void discoverOauthDependencyField_multipleArenaDependencyFields_throwsIllegalStateException() {
    IllegalStateException error =
        assertThrows(
            IllegalStateException.class,
            () -> invokeDiscoverOauthDependencyField(MultipleOauthDependencyFieldsHost.Fixture.class));

    assertTrue(error.getMessage().contains("found 2"));
  }

  private static Field invokeDiscoverOauthDependencyField(Class<?> fixtureClass) throws Exception {
    Method method =
        OauthSigner.class.getDeclaredMethod("discoverOauthDependencyField", Class.class);
    method.setAccessible(true);
    try {
      return (Field) method.invoke(null, fixtureClass);
    } catch (InvocationTargetException e) {
      if (e.getCause() instanceof RuntimeException re) {
        throw re;
      }
      throw e;
    }
  }

  private static final class NoOauthDependencyFieldHost {
    static final class Fixture {}
  }

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

  private static final class SingleOauthDependencyFieldHost {
    static final class Fixture {
      @ArenaDependency
      static final OauthDependency OAUTH =
          new OauthDependency(JsonNodeFactory.instance.objectNode().put("identifier", "oauth"));
    }
  }

  private static class BaseFixtureWithOauthDependency {
    @ArenaDependency
    static final OauthDependency OAUTH =
        new OauthDependency(JsonNodeFactory.instance.objectNode().put("identifier", "oauth-base"));
  }

  private static final class InheritedOauthDependencyFieldHost {
    static final class SubFixture extends BaseFixtureWithOauthDependency {}
  }
}
