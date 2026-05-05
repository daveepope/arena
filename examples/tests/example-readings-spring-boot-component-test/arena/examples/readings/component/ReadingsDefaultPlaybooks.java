package arena.examples.readings.component;

import arena.junit.playbook.ActivePlaybook;
import arena.junit.playbook.ArenaPlaybookSupplier;
import arena.junit.playbook.ManagedMssqlPlaybook;
import arena.junit.playbook.ManagedMssqlPlaybookBuilder;
import java.lang.reflect.Field;
import org.junit.jupiter.api.extension.ExtensionContext;

public final class ReadingsDefaultPlaybooks implements ArenaPlaybookSupplier {

  @Override
  public ActivePlaybook[] supply(ExtensionContext context) {
    try {
      Field f = context.getRequiredTestClass().getDeclaredField("readings");
      f.setAccessible(true);
      ReadingsArenaFixture fx = (ReadingsArenaFixture) f.get(null);
      ManagedMssqlPlaybook validationDb =
          new ManagedMssqlPlaybookBuilder("spring-validation-db-scoped", fx.mssqlIdentifier())
              .build();
      return new ActivePlaybook[] {validationDb, fx.localstackSessionPlaybook()};
    } catch (Exception e) {
      throw new IllegalStateException(e);
    }
  }
}
