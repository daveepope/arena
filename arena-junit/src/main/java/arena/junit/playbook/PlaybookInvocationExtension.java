package arena.junit.playbook;

import arena.junit.ArenaExtension;
import arena.junit.OpenArena;

import java.lang.reflect.AnnotatedElement;
import java.lang.reflect.Method;
import java.util.ArrayList;
import java.util.List;
import org.junit.jupiter.api.extension.AfterAllCallback;
import org.junit.jupiter.api.extension.AfterEachCallback;
import org.junit.jupiter.api.extension.BeforeAllCallback;
import org.junit.jupiter.api.extension.BeforeEachCallback;
import org.junit.jupiter.api.extension.ExtensionContext;
import org.junit.jupiter.api.extension.ParameterContext;
import org.junit.jupiter.api.extension.ParameterResolver;

public final class PlaybookInvocationExtension
    implements BeforeAllCallback,
        BeforeEachCallback,
        AfterEachCallback,
        AfterAllCallback,
        ParameterResolver {

  private static final ExtensionContext.Namespace NS =
      ExtensionContext.Namespace.create("arena.junit.playbook.scope");

  private static final String CLASS_SCOPE_KEY = "classScope";
  private static final String CLASS_SCOPE_ATTEMPTED = "classScopeAttempted";
  private static final String METHOD_SCOPE_KEY = "methodScope";

  @Override
  public void beforeAll(ExtensionContext context) {
    Class<?> testClass = context.getRequiredTestClass();
    Class<? extends Playbook>[] classes = classesFrom(testClass);
    if (classes.length == 0) {
      return;
    }
    tryOpenClassScope(context, classes);
  }

  @Override
  public void beforeEach(ExtensionContext context) {
    Method testMethod = context.getTestMethod().orElse(null);
    if (testMethod != null) {
      Class<? extends Playbook>[] methodClasses = classesFrom(testMethod);
      if (methodClasses.length > 0) {
        OpenArena arena = resolveOpenArena(context);
        PlaybookScope scope = openScope(arena, methodClasses);
        context.getStore(NS).put(METHOD_SCOPE_KEY, scope);
        return;
      }
    }
    Class<?> testClass = context.getRequiredTestClass();
    Class<? extends Playbook>[] classes = classesFrom(testClass);
    if (classes.length == 0) {
      return;
    }
    ExtensionContext classContext = context.getParent().orElse(context.getRoot());
    ExtensionContext.Store store = classContext.getStore(NS);
    if (store.get(CLASS_SCOPE_KEY) != null) {
      return;
    }
    tryOpenClassScope(classContext, classes);
    if (store.get(CLASS_SCOPE_KEY) == null) {
      throw new IllegalStateException(
          "@Playbook: unable to open class-scope playbooks for "
              + testClass.getName()
              + " (open arena not initialized)");
    }
  }

  @Override
  public void afterEach(ExtensionContext context) {
    PlaybookScope methodScope = context.getStore(NS).remove(METHOD_SCOPE_KEY, PlaybookScope.class);
    if (methodScope != null) {
      methodScope.close();
    }
  }

  @Override
  public void afterAll(ExtensionContext context) {
    ExtensionContext.Store store = context.getStore(NS);
    PlaybookScope scope = store.remove(CLASS_SCOPE_KEY, PlaybookScope.class);
    if (scope != null) {
      scope.close();
    }
  }

  @Override
  public boolean supportsParameter(
      ParameterContext parameterContext, ExtensionContext extensionContext) {
    return ActiveHttpPlaybook.class.isAssignableFrom(parameterContext.getParameter().getType());
  }

  @Override
  public Object resolveParameter(
      ParameterContext parameterContext, ExtensionContext extensionContext) {
    PlaybookScope scope = extensionContext.getStore(NS).get(METHOD_SCOPE_KEY, PlaybookScope.class);
    if (scope == null) {
      throw new IllegalStateException(
          "ActiveHttpPlaybook parameter requires stacked @Playbook on the test method");
    }
    List<ActiveHttpPlaybook> httpActives = new ArrayList<>();
    for (ActivePlaybook active : scope.actives()) {
      if (active instanceof ActiveHttpPlaybook http) {
        httpActives.add(http);
      }
    }
    if (httpActives.size() != 1) {
      throw new IllegalStateException(
          "expected exactly one ActiveHttpPlaybook from stacked @Playbook markers");
    }
    return httpActives.getFirst();
  }

  private static void tryOpenClassScope(
      ExtensionContext classContext, Class<? extends Playbook>[] classes) {
    ExtensionContext.Store store = classContext.getStore(NS);
    if (store.get(CLASS_SCOPE_KEY) != null) {
      return;
    }
    OpenArena arena;
    try {
      arena = resolveOpenArena(classContext);
    } catch (Exception ignored) {
      store.put(CLASS_SCOPE_ATTEMPTED, Boolean.TRUE);
      return;
    }
    if (arena == null || arena.handle() == null) {
      store.put(CLASS_SCOPE_ATTEMPTED, Boolean.TRUE);
      return;
    }
    PlaybookScope scope = openScope(arena, classes);
    store.put(CLASS_SCOPE_KEY, scope);
  }

  private static Class<? extends Playbook>[] classesFrom(AnnotatedElement element) {
    arena.junit.Playbook[] anns = element.getAnnotationsByType(arena.junit.Playbook.class);
    if (anns.length == 0) {
      return new Class[0];
    }
    Class<? extends Playbook>[] out = new Class[anns.length];
    for (int i = 0; i < anns.length; i++) {
      out[i] = anns[i].value();
    }
    return out;
  }

  private static PlaybookScope openScope(OpenArena arena, Class<? extends Playbook>[] classes) {
    List<ActivePlaybook> opened = new ArrayList<>();
    try {
      for (Class<? extends Playbook> klass : classes) {
        Playbook pb = arena.playbook(klass);
        if (pb == null) {
          throw new IllegalStateException(
              "@Playbook: no playbook of class "
                  + klass.getName()
                  + " is registered on any match");
        }
        Boolean execOnDependencyStart = arena.playbookExecOnDependencyStart(klass);
        if (Boolean.TRUE.equals(execOnDependencyStart)) {
          throw new IllegalStateException(
              "@Playbook: playbook "
                  + klass.getName()
                  + " was registered with execOnDependencyStart=true and cannot be scoped per-test");
        }
        opened.add(pb.run(arena));
      }
    } catch (RuntimeException e) {
      closeAll(opened);
      throw e;
    }
    return new PlaybookScope(opened);
  }

  private static void closeAll(List<ActivePlaybook> opened) {
    for (int i = opened.size() - 1; i >= 0; i--) {
      try {
        opened.get(i).close();
      } catch (RuntimeException ignored) {
      }
    }
  }

  private static OpenArena resolveOpenArena(ExtensionContext ctx) {
    return ArenaExtension.openArenaFor(ctx.getRequiredTestClass());
  }

  static final class PlaybookScope {
    private final List<ActivePlaybook> opened;

    PlaybookScope(List<ActivePlaybook> opened) {
      this.opened = opened;
    }

    void close() {
      closeAll(opened);
    }

    List<ActivePlaybook> actives() {
      return opened;
    }
  }
}
