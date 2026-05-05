package dev.arena.junit.ffi;
import com.sun.jna.Library;
import com.sun.jna.Native;
import com.sun.jna.Pointer;
import com.sun.jna.ptr.PointerByReference;

interface ArenaNativeLib extends Library {
  Pointer arena_open(String name, String configJson, PointerByReference errOut);

  void arena_close(Pointer handle);

  int arena_soft_reset(Pointer handle, String dependencyIdentifier, PointerByReference errOut);

  int arena_hard_reset(Pointer handle, String dependencyIdentifier, PointerByReference errOut);

  void arena_free_string(Pointer p);

  Pointer arena_oauth_loopback_tls_pem_json(PointerByReference errOut);

  Pointer arena_http_playbook_open(Pointer arena, String specJson, PointerByReference errOut);

  int arena_http_playbook_close(Pointer playbook, PointerByReference errOut);

  int arena_http_playbook_verify(Pointer playbook, String specJson, PointerByReference errOut);

  Pointer arena_mssql_playbook_open(Pointer arena, String specJson, PointerByReference errOut);

  int arena_mssql_playbook_close(Pointer playbook, PointerByReference errOut);

  int arena_mssql_playbook_verify(Pointer playbook, String specJson, PointerByReference errOut);

  Pointer arena_localstack_playbook_open(Pointer arena, String specJson, PointerByReference errOut);

  int arena_localstack_playbook_close(Pointer playbook, PointerByReference errOut);
}

final class ArenaNativeHolder {
  static final ArenaNativeLib LIB;

  static {
    String path = ArenaPaths.resolveArenaSharedLibrary();
    if (path == null || path.isEmpty()) {
      LIB = null;
    } else {
      LIB = Native.load(path, ArenaNativeLib.class);
    }
  }

  private ArenaNativeHolder() {}
}
