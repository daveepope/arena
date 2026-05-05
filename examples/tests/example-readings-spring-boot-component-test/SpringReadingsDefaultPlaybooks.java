package dev.arena.examples.readings.component;

import dev.arena.junit.playbook.ActivePlaybook;
import dev.arena.junit.playbook.ArenaPlaybookSupplier;
import dev.arena.junit.playbook.ManagedMssqlPlaybook;
import dev.arena.junit.playbook.ManagedMssqlPlaybookBuilder;
import java.lang.reflect.Field;
import org.junit.jupiter.api.extension.ExtensionContext;

public final class SpringReadingsDefaultPlaybooks implements ArenaPlaybookSupplier {

  @Override
  public ActivePlaybook[] supply(ExtensionContext context) {
    try {
      Field f = context.getRequiredTestClass().getDeclaredField("springReadings");
      f.setAccessible(true);
      ReadingsSpringBootArenaFixture fx = (ReadingsSpringBootArenaFixture) f.get(null);
      ManagedMssqlPlaybook validationDb =
          new ManagedMssqlPlaybookBuilder("spring-validation-db-scoped", fx.mssqlIdentifier())
              .build();
      return new ActivePlaybook[] {validationDb, fx.localstackSessionPlaybook()};
    } catch (Exception e) {
      throw new IllegalStateException(e);
    }
  }
}
