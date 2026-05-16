package arena.junit.playbook;

import arena.junit.OpenArena;
import java.lang.reflect.Field;
import java.lang.reflect.Modifier;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import org.junit.jupiter.api.extension.ExtensionContext;
import org.junit.jupiter.api.extension.InvocationInterceptor;
import org.junit.jupiter.api.extension.ReflectiveInvocationContext;
import org.junit.jupiter.api.extension.RegisterExtension;

final class PlaybookInvocationExtension implements InvocationInterceptor {

  @Override
  public void interceptTestMethod(
      Invocation<Void> invocation,
      ReflectiveInvocationContext<java.lang.reflect.Method> invocationContext,
      ExtensionContext extensionContext)
      throws Throwable {
    ArenaPlaybooks ann =
        invocationContext.getExecutable().getAnnotation(ArenaPlaybooks.class);
    if (ann == null) {
      invocation.proceed();
      return;
    }
    Class<? extends ArenaPlaybookSupplier>[] types = ann.value();
    if (types.length == 0) {
      invocation.proceed();
      return;
    }
    OpenArena arena = resolveArena(extensionContext);
    List<Playbook> list = new ArrayList<>();
    for (Class<? extends ArenaPlaybookSupplier> t : types) {
      ArenaPlaybookSupplier sup = instantiateSupplier(t);
      list.addAll(Arrays.asList(sup.supply(extensionContext)));
    }
    Playbook[] arr = list.toArray(Playbook[]::new);
    try (ActivePlaybooks scope = ActivePlaybooks.open(arena, arr)) {
      invocation.proceed();
    }
  }

  private static ArenaPlaybookSupplier instantiateSupplier(Class<? extends ArenaPlaybookSupplier> c)
      throws Exception {
    if (c.isEnum()) {
      Object[] cs = c.getEnumConstants();
      if (cs == null || cs.length != 1) {
        throw new IllegalStateException(
            "ArenaPlaybookSupplier enum must declare exactly one constant: " + c.getName());
      }
      return (ArenaPlaybookSupplier) cs[0];
    }
    return c.getDeclaredConstructor().newInstance();
  }

  private static OpenArena resolveArena(ExtensionContext ctx) throws Exception {
    Object instance = ctx.getTestInstance().orElse(null);
    Class<?> testClass = ctx.getRequiredTestClass();
    List<Field> matches = new ArrayList<>();
    for (Class<?> c = testClass; c != null && c != Object.class; c = c.getSuperclass()) {
      for (Field f : c.getDeclaredFields()) {
        if (f.isAnnotationPresent(RegisterExtension.class)
            && ArenaSession.class.isAssignableFrom(f.getType())) {
          matches.add(f);
        }
      }
    }
    if (matches.isEmpty()) {
      throw new IllegalStateException(
          "@ArenaPlaybooks requires exactly one @RegisterExtension field whose type implements "
              + ArenaSession.class.getSimpleName());
    }
    if (matches.size() > 1) {
      throw new IllegalStateException(
          "@ArenaPlaybooks: multiple @RegisterExtension ArenaSession fields on "
              + testClass.getName());
    }
    Field f = matches.get(0);
    f.setAccessible(true);
    Object ext =
        Modifier.isStatic(f.getModifiers()) ? f.get(null) : (instance != null ? f.get(instance) : null);
    if (ext == null) {
      throw new IllegalStateException(
          "@ArenaPlaybooks: ArenaSession extension field not initialized: " + f.getName());
    }
    return ((ArenaSession) ext).arena();
  }
}
