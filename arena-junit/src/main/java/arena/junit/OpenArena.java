package arena.junit;

import arena.junit.ffi.ArenaBindingError;
import arena.junit.ffi.ArenaBindings;
import arena.junit.ffi.ArenaLogbackFlush;
import arena.junit.ffi.ArenaStatus;
import arena.junit.lifecycle.ArenaLifecycleError;
import arena.junit.lifecycle.ArenaState;
import arena.junit.lifecycle.LifecycleLog;
import arena.junit.match.Match;
import arena.junit.playbook.Playbook;
import com.sun.jna.Pointer;
import java.util.List;

public final class OpenArena {
  private Pointer handle;
  private volatile long dispatcherLoggingTargetToken;
  private final List<Match> matches;

  OpenArena(Pointer handle, long dispatcherLoggingTargetToken, List<Match> matches) {
    this.handle = handle;
    this.dispatcherLoggingTargetToken = dispatcherLoggingTargetToken;
    this.matches = List.copyOf(matches);
  }

  public Pointer handle() {
    return handle;
  }

  public List<Match> matches() {
    return matches;
  }

  public Playbook playbook(Class<? extends Playbook> klass) {
    for (Match m : matches) {
      Playbook pb = m.playbook(klass);
      if (pb != null) {
        return pb;
      }
    }
    return null;
  }

  public Boolean playbookExecOnDependencyStart(Class<? extends Playbook> klass) {
    for (Match m : matches) {
      Boolean flag = m.execOnDependencyStart(klass);
      if (flag != null) {
        return flag;
      }
    }
    return null;
  }

  public void close() {
    Pointer h = handle;
    if (h == null || Pointer.nativeValue(h) == 0) {
      long t = dispatcherLoggingTargetToken;
      if (t != 0L) {
        ArenaBindings.unregisterDispatcherLoggingTarget(t);
        dispatcherLoggingTargetToken = 0L;
      }
      return;
    }
    handle = null;
    String stateDocument = null;
    try {
      try {
        stateDocument = ArenaBindings.arenaClose(h);
      } catch (ArenaBindingError e) {
        throw ArenaLifecycleError.from(e);
      }
    } finally {
      ArenaLogbackFlush.flushIfPresent();
      long tok = dispatcherLoggingTargetToken;
      if (tok != 0L) {
        ArenaBindings.unregisterDispatcherLoggingTarget(tok);
        dispatcherLoggingTargetToken = 0L;
      }
    }
    if (stateDocument != null) {
      LifecycleLog.logClosingSummaryDocument(stateDocument);
    }
  }

  public ArenaState state() {
    return ArenaState.parse(ArenaBindings.arenaStateJson(handle));
  }

  public ArenaStatus softReset(String dependencyIdentifier) {
    return ArenaBindings.softReset(handle, dependencyIdentifier);
  }

  public ArenaStatus hardReset(String dependencyIdentifier) {
    return ArenaBindings.hardReset(handle, dependencyIdentifier);
  }
}
