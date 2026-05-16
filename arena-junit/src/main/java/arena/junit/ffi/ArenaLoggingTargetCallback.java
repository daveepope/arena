package arena.junit.ffi;

import com.sun.jna.Callback;
import com.sun.jna.Pointer;

public interface ArenaLoggingTargetCallback extends Callback {
  void invoke(
      int level,
      Pointer targetUtf8,
      long tsNanos,
      Pointer messageUtf8,
      Pointer callerFileUtf8,
      int callerLine,
      Pointer userData);
}
