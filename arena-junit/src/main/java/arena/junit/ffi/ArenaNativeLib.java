package arena.junit.ffi;
import com.sun.jna.Library;
import com.sun.jna.Native;
import com.sun.jna.Pointer;
import com.sun.jna.ptr.PointerByReference;
import java.util.Map;

interface ArenaNativeLib extends Library {
  void arena_set_log_level(int level);

  Pointer arena_open(String name, String configJson, PointerByReference errOut);

  void arena_close(Pointer handle);

  long arena_add_log_target(ArenaLoggingTargetCallback callback, Pointer userData);

  void arena_remove_log_target(long token);

  Pointer arena_dispatcher_default_logging_target_logger_name_utf8();

  int arena_dispatcher_default_logging_target_publish_level(int level);

  void arena_dispatcher_dependency_allow_json_set(String jsonUtf8Nullable);

  void arena_dispatcher_component_allow_json_set(String jsonUtf8Nullable);

  int arena_soft_reset(Pointer handle, String dependencyIdentifier, PointerByReference errOut);

  int arena_hard_reset(Pointer handle, String dependencyIdentifier, PointerByReference errOut);

  void arena_free_string(Pointer p);

  Pointer arena_oauth_loopback_tls_pem_json(PointerByReference errOut);

  Pointer arena_match_playbook_run(Pointer arena, String identifier, PointerByReference errOut);

  int arena_active_playbook_drop(Pointer handle, PointerByReference errOut);

  Pointer arena_http_playbook_open(Pointer arena, String specJson, PointerByReference errOut);

  int arena_http_playbook_verify(Pointer handle, String specJson, PointerByReference errOut);

  int arena_mssql_playbook_verify(Pointer handle, String specJson, PointerByReference errOut);

  int arena_postgres_playbook_verify(Pointer handle, String specJson, PointerByReference errOut);
}

final class ArenaNativeHolder {
  static final ArenaNativeLib LIB;

  static {
    // The native side (Rust) requires UTF-8; without this, JNA falls back to the
    // JVM's platform-default encoding for String<->native marshaling, which is
    // UTF-8 on Linux/macOS but commonly a Windows codepage (e.g. Cp1252) on Windows.
    Map<String, Object> options = Map.of(Library.OPTION_STRING_ENCODING, "UTF-8");
    String path = ArenaPaths.resolveArenaSharedLibrary();
    if (path != null && !path.isEmpty()) {
      LIB = Native.load(path, ArenaNativeLib.class, options);
    } else {
      LIB = ArenaPaths.loadFromClasspath(ArenaNativeLib.class);
    }
  }

  private ArenaNativeHolder() {}
}
