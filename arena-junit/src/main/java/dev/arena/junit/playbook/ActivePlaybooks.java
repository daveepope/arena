package dev.arena.junit.playbook;
import dev.arena.junit.OpenArena;

import java.util.ArrayList;
import java.util.List;

public final class ActivePlaybooks implements AutoCloseable {
  private final List<AutoCloseable> opened;

  private ActivePlaybooks(List<AutoCloseable> opened) {
    this.opened = opened;
  }

  public static ActivePlaybooks open(OpenArena arena, ActivePlaybook... playbooks) {
    if (playbooks.length == 0) {
      throw new IllegalArgumentException("active playbooks requires at least one playbook");
    }
    List<AutoCloseable> list = new ArrayList<>();
    try {
      for (ActivePlaybook pb : playbooks) {
        list.add(pb.enter(arena));
      }
    } catch (RuntimeException e) {
      for (AutoCloseable c : list) {
        try {
          c.close();
        } catch (Exception ignored) {
        }
      }
      throw e;
    }
    return new ActivePlaybooks(list);
  }

  @Override
  public void close() throws Exception {
    Exception first = null;
    for (int i = opened.size() - 1; i >= 0; i--) {
      try {
        opened.get(i).close();
      } catch (Exception e) {
        if (first == null) {
          first = e;
        }
      }
    }
    if (first != null) {
      throw first;
    }
  }
}
