package arena.junit.ffi;

import com.sun.jna.Callback;
import com.sun.jna.Pointer;

public interface ArenaLifecycleObserverCallback extends Callback {
  void invoke(Pointer stateJsonUtf8, Pointer userData);
}
