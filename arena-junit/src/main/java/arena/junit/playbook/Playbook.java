package arena.junit.playbook;

import arena.junit.OpenArena;

public interface Playbook {
  AutoCloseable enter(OpenArena arena);
}
