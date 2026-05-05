package dev.arena.junit.playbook;
import dev.arena.junit.OpenArena;

public interface ActivePlaybook {
  AutoCloseable enter(OpenArena arena);
}
