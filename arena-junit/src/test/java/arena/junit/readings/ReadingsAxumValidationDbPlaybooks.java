package arena.junit.readings;

import arena.junit.playbook.ActivePlaybook;
import arena.junit.playbook.ArenaPlaybookSupplier;
import arena.junit.playbook.ManagedMssqlPlaybook;
import arena.junit.playbook.ManagedMssqlPlaybookBuilder;
import java.lang.reflect.Field;
import org.junit.jupiter.api.extension.ExtensionContext;

public final class ReadingsAxumValidationDbPlaybooks implements ArenaPlaybookSupplier {

  @Override
  public ActivePlaybook[] supply(ExtensionContext context) {
    try {
      Field f = context.getRequiredTestClass().getDeclaredField("readingsArena");
      f.setAccessible(true);
      ReadingsArenaSessionFixture ar = (ReadingsArenaSessionFixture) f.get(null);
      return new ActivePlaybook[] {
        new ManagedMssqlPlaybookBuilder(
                ReadingsArenaConfig.PLAYBOOK_VALIDATION_DB_SCOPED, ar.mssqlIdentifier())
            .build()
      };
    } catch (Exception e) {
      throw new IllegalStateException(e);
    }
  }
}
