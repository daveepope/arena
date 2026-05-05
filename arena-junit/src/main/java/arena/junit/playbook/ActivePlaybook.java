package arena.junit.playbook;
import arena.junit.OpenArena;

public interface ActivePlaybook {
  AutoCloseable enter(OpenArena arena);
}
