using System;
using System.Runtime.InteropServices;

namespace ArenaXunit.Ffi;

internal static class ArenaNativeLib
{
    private const string LibName = "arena_ffi";
    private static IntPtr _libHandle = IntPtr.Zero;
    private static bool _initialized = false;

    private delegate IntPtr arena_open_fn(string name, string configJson, out IntPtr errOut);
    private delegate void arena_close_fn(IntPtr handle);
    private delegate int arena_soft_reset_fn(IntPtr handle, string dependencyIdentifier, out IntPtr errOut);
    private delegate int arena_hard_reset_fn(IntPtr handle, string dependencyIdentifier, out IntPtr errOut);
    private delegate void arena_set_log_level_fn(int level);
    private delegate ulong arena_add_log_target_fn(ArenaLogCallback callback, IntPtr userData);
    private delegate void arena_remove_log_target_fn(ulong token);
    private delegate IntPtr arena_dispatcher_default_logging_target_logger_name_utf8_fn();
    private delegate int arena_dispatcher_default_logging_target_publish_level_fn(int level);
    private delegate void arena_dispatcher_dependency_allow_json_set_fn(string jsonUtf8Nullable);
    private delegate void arena_dispatcher_component_allow_json_set_fn(string jsonUtf8Nullable);
    private delegate void arena_free_string_fn(IntPtr ptr);
    private delegate IntPtr arena_oauth_loopback_tls_pem_json_fn(out IntPtr errOut);
    private delegate IntPtr arena_match_playbook_run_fn(IntPtr arena, string identifier, out IntPtr errOut);
    private delegate int arena_active_playbook_drop_fn(IntPtr handle, out IntPtr errOut);
    private delegate IntPtr arena_http_playbook_open_fn(IntPtr arena, string specJson, out IntPtr errOut);
    private delegate int arena_http_playbook_verify_fn(IntPtr handle, string specJson, out IntPtr errOut);
    private delegate int arena_mssql_playbook_verify_fn(IntPtr handle, string specJson, out IntPtr errOut);

    private static arena_open_fn? _arena_open;
    private static arena_close_fn? _arena_close;
    private static arena_soft_reset_fn? _arena_soft_reset;
    private static arena_hard_reset_fn? _arena_hard_reset;
    private static arena_set_log_level_fn? _arena_set_log_level;
    private static arena_add_log_target_fn? _arena_add_log_target;
    private static arena_remove_log_target_fn? _arena_remove_log_target;
    private static arena_dispatcher_default_logging_target_logger_name_utf8_fn? _arena_dispatcher_default_logging_target_logger_name_utf8;
    private static arena_dispatcher_default_logging_target_publish_level_fn? _arena_dispatcher_default_logging_target_publish_level;
    private static arena_dispatcher_dependency_allow_json_set_fn? _arena_dispatcher_dependency_allow_json_set;
    private static arena_dispatcher_component_allow_json_set_fn? _arena_dispatcher_component_allow_json_set;
    private static arena_free_string_fn? _arena_free_string;
    private static arena_oauth_loopback_tls_pem_json_fn? _arena_oauth_loopback_tls_pem_json;
    private static arena_match_playbook_run_fn? _arena_match_playbook_run;
    private static arena_active_playbook_drop_fn? _arena_active_playbook_drop;
    private static arena_http_playbook_open_fn? _arena_http_playbook_open;
    private static arena_http_playbook_verify_fn? _arena_http_playbook_verify;
    private static arena_mssql_playbook_verify_fn? _arena_mssql_playbook_verify;

    static ArenaNativeLib()
    {
        Init();
    }

    internal static void Init()
    {
        if (_initialized)
            return;

        lock (typeof(ArenaNativeLib))
        {
            if (_initialized)
                return;

            var path = ArenaPaths.ResolveArenaSharedLibrary();
            if (string.IsNullOrEmpty(path))
            {
                throw new ArenaBindingError(
                    $"arena shared library not found (set ARENA_FFI_LIB or use Bazel runfiles)");
            }

            try
            {
                _libHandle = NativeLibrary.Load(path);
            }
            catch (Exception ex)
            {
                throw new ArenaBindingError(
                    $"failed to load arena shared library from '{path}': {ex.Message}", ex);
            }

            LoadFunction(out _arena_open, "arena_open");
            LoadFunction(out _arena_close, "arena_close");
            LoadFunction(out _arena_soft_reset, "arena_soft_reset");
            LoadFunction(out _arena_hard_reset, "arena_hard_reset");
            LoadFunction(out _arena_set_log_level, "arena_set_log_level");
            LoadFunction(out _arena_add_log_target, "arena_add_log_target");
            LoadFunction(out _arena_remove_log_target, "arena_remove_log_target");
            LoadFunction(out _arena_dispatcher_default_logging_target_logger_name_utf8, "arena_dispatcher_default_logging_target_logger_name_utf8");
            LoadFunction(out _arena_dispatcher_default_logging_target_publish_level, "arena_dispatcher_default_logging_target_publish_level");
            LoadFunction(out _arena_dispatcher_dependency_allow_json_set, "arena_dispatcher_dependency_allow_json_set");
            LoadFunction(out _arena_dispatcher_component_allow_json_set, "arena_dispatcher_component_allow_json_set");
            LoadFunction(out _arena_free_string, "arena_free_string");
            LoadFunction(out _arena_oauth_loopback_tls_pem_json, "arena_oauth_loopback_tls_pem_json");
            LoadFunction(out _arena_match_playbook_run, "arena_match_playbook_run");
            LoadFunction(out _arena_active_playbook_drop, "arena_active_playbook_drop");
            LoadFunction(out _arena_http_playbook_open, "arena_http_playbook_open");
            LoadFunction(out _arena_http_playbook_verify, "arena_http_playbook_verify");
            LoadFunction(out _arena_mssql_playbook_verify, "arena_mssql_playbook_verify");

            _initialized = true;
        }
    }

    private static void LoadFunction<T>(out T? func, string name) where T : Delegate
    {
        var ptr = NativeLibrary.GetExport(_libHandle, name);
        if (ptr == IntPtr.Zero)
        {
            throw new ArenaBindingError($"export '{name}' not found in arena shared library");
        }
        func = Marshal.GetDelegateForFunctionPointer<T>(ptr);
    }

    internal static IntPtr arena_open(string name, string configJson, out IntPtr errOut) =>
        _arena_open!.Invoke(name, configJson, out errOut);

    internal static void arena_close(IntPtr handle) =>
        _arena_close!.Invoke(handle);

    internal static int arena_soft_reset(IntPtr handle, string dependencyIdentifier, out IntPtr errOut) =>
        _arena_soft_reset!.Invoke(handle, dependencyIdentifier, out errOut);

    internal static int arena_hard_reset(IntPtr handle, string dependencyIdentifier, out IntPtr errOut) =>
        _arena_hard_reset!.Invoke(handle, dependencyIdentifier, out errOut);

    internal static void arena_set_log_level(int level) =>
        _arena_set_log_level!.Invoke(level);

    internal static ulong arena_add_log_target(ArenaLogCallback callback, IntPtr userData) =>
        _arena_add_log_target!.Invoke(callback, userData);

    internal static void arena_remove_log_target(ulong token) =>
        _arena_remove_log_target!.Invoke(token);

    internal static IntPtr arena_dispatcher_default_logging_target_logger_name_utf8() =>
        _arena_dispatcher_default_logging_target_logger_name_utf8!.Invoke();

    internal static int arena_dispatcher_default_logging_target_publish_level(int level) =>
        _arena_dispatcher_default_logging_target_publish_level!.Invoke(level);

    internal static void arena_dispatcher_dependency_allow_json_set(string jsonUtf8Nullable) =>
        _arena_dispatcher_dependency_allow_json_set!.Invoke(jsonUtf8Nullable);

    internal static void arena_dispatcher_component_allow_json_set(string jsonUtf8Nullable) =>
        _arena_dispatcher_component_allow_json_set!.Invoke(jsonUtf8Nullable);

    internal static void arena_free_string(IntPtr ptr) =>
        _arena_free_string!.Invoke(ptr);

    internal static IntPtr arena_oauth_loopback_tls_pem_json(out IntPtr errOut) =>
        _arena_oauth_loopback_tls_pem_json!.Invoke(out errOut);

    internal static IntPtr arena_match_playbook_run(IntPtr arena, string identifier, out IntPtr errOut) =>
        _arena_match_playbook_run!.Invoke(arena, identifier, out errOut);

    internal static int arena_active_playbook_drop(IntPtr handle, out IntPtr errOut) =>
        _arena_active_playbook_drop!.Invoke(handle, out errOut);

    internal static IntPtr arena_http_playbook_open(IntPtr arena, string specJson, out IntPtr errOut) =>
        _arena_http_playbook_open!.Invoke(arena, specJson, out errOut);

    internal static int arena_http_playbook_verify(IntPtr handle, string specJson, out IntPtr errOut) =>
        _arena_http_playbook_verify!.Invoke(handle, specJson, out errOut);

    internal static int arena_mssql_playbook_verify(IntPtr handle, string specJson, out IntPtr errOut) =>
        _arena_mssql_playbook_verify!.Invoke(handle, specJson, out errOut);
}
