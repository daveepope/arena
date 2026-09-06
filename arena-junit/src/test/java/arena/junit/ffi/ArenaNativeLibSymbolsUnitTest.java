package arena.junit.ffi;

import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.sun.jna.NativeLibrary;
import java.lang.reflect.Method;
import java.util.ArrayList;
import java.util.List;
import org.junit.jupiter.api.Test;

class ArenaNativeLibSymbolsUnitTest {

  private static List<String> declaredSymbols() {
    List<String> names = new ArrayList<>();
    for (Method method : ArenaNativeLib.class.getDeclaredMethods()) {
      names.add(method.getName());
    }
    return names;
  }

  @Test
  void declaredSymbolsEveryBoundMethodResolvesInTheNative() {
    String path = ArenaPaths.resolveArenaSharedLibrary();
    assertTrue(
        path != null && !path.isEmpty(),
        "arena shared library must be resolvable for this test");
    NativeLibrary library = NativeLibrary.getInstance(path);

    List<String> declared = declaredSymbols();
    assertFalse(declared.isEmpty(), "ArenaNativeLib must declare at least one bound method");
    for (String symbol : declared) {
      assertNotNull(library.getFunction(symbol), symbol + " is not exported by the native");
    }
  }
}
