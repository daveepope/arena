package arena.junit.playbook;

import org.junit.jupiter.api.extension.ExtensionContext;

public interface ArenaPlaybookSupplier {
  ActivePlaybook[] supply(ExtensionContext context);
}
