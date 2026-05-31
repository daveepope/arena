package arena.junit.playbook;

import arena.junit.OpenArena;

public interface Playbook {
  String identifier();

  ActivePlaybook run(OpenArena arena);
}
