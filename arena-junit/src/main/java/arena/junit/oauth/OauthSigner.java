package arena.junit.oauth;

import arena.junit.ArenaDependency;
import arena.junit.ArenaExtension;
import arena.junit.OpenArena;
import java.lang.reflect.Field;
import java.lang.reflect.Modifier;
import java.util.ArrayList;
import java.util.List;
import java.util.concurrent.ConcurrentHashMap;

public final class OauthSigner {
  private static final ConcurrentHashMap<Class<?>, Field> FIELD_CACHE = new ConcurrentHashMap<>();

  private final OpenArena arena;
  private final OauthDependency dependency;

  OauthSigner(OpenArena arena, OauthDependency dependency) {
    this.arena = arena;
    this.dependency = dependency;
  }

  public String sign(Provider provider, String claimsJson) {
    return dependency.signClaims(arena, provider, claimsJson);
  }

  public static OauthSigner forFixture(Class<?> fixtureClass) {
    OauthDependency dependency = readOauthDependency(oauthDependencyFieldFor(fixtureClass));
    return new OauthSigner(ArenaExtension.openArenaFor(fixtureClass), dependency);
  }

  private static OauthDependency readOauthDependency(Field field) {
    try {
      return (OauthDependency) field.get(null);
    } catch (IllegalAccessException e) {
      throw new IllegalStateException("failed to read OauthDependency field", e);
    }
  }

  private static Field oauthDependencyFieldFor(Class<?> fixtureClass) {
    return FIELD_CACHE.computeIfAbsent(fixtureClass, OauthSigner::discoverOauthDependencyField);
  }

  private static Field discoverOauthDependencyField(Class<?> fixtureClass) {
    List<Field> matches = new ArrayList<>();
    for (Class<?> current = fixtureClass;
        current != null && current != Object.class;
        current = current.getSuperclass()) {
      for (Field field : current.getDeclaredFields()) {
        if (Modifier.isStatic(field.getModifiers())
            && field.isAnnotationPresent(ArenaDependency.class)
            && OauthDependency.class.isAssignableFrom(field.getType())) {
          matches.add(field);
        }
      }
    }
    if (matches.size() != 1) {
      throw new IllegalStateException(
          "expected exactly one static @ArenaDependency OauthDependency field on "
              + fixtureClass.getName()
              + " or a superclass, found "
              + matches.size());
    }
    Field field = matches.get(0);
    field.setAccessible(true);
    return field;
  }
}
