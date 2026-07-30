using System;
using System.Runtime.InteropServices;

namespace ArenaXunit.Ffi;

internal static class ArenaNativeLib
{
    private const string LibName = "arena_ffi_shared";

    [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
    internal static extern IntPtr arena_open(string name, string configJson, out IntPtr errOut);

    [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
    internal static extern void arena_close(IntPtr handle);

    [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
    internal static extern int arena_soft_reset(IntPtr handle, string dependencyIdentifier, out IntPtr errOut);

    [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
    internal static extern int arena_hard_reset(IntPtr handle, string dependencyIdentifier, out IntPtr errOut);

    [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
    internal static extern void arena_set_log_level(int level);

    [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
    internal static extern ulong arena_add_log_target(ArenaLogCallback callback, IntPtr userData);

    [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
    internal static extern void arena_remove_log_target(ulong token);

    [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
    internal static extern IntPtr arena_dispatcher_default_logging_target_logger_name_utf8();

    [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
    internal static extern int arena_dispatcher_default_logging_target_publish_level(int level);

    [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
    internal static extern void arena_dispatcher_dependency_allow_json_set(string jsonUtf8Nullable);

    [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
    internal static extern void arena_dispatcher_component_allow_json_set(string jsonUtf8Nullable);

    [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
    internal static extern void arena_free_string(IntPtr ptr);

    [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
    internal static extern IntPtr arena_oauth_loopback_tls_pem_json(out IntPtr errOut);

    [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
    internal static extern IntPtr arena_match_playbook_run(IntPtr arena, string identifier, out IntPtr errOut);

    [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
    internal static extern int arena_active_playbook_drop(IntPtr handle, out IntPtr errOut);

    [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
    internal static extern IntPtr arena_http_playbook_open(IntPtr arena, string specJson, out IntPtr errOut);

    [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
    internal static extern int arena_http_playbook_verify(IntPtr handle, string specJson, out IntPtr errOut);

    [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
    internal static extern int arena_mssql_playbook_verify(IntPtr handle, string specJson, out IntPtr errOut);
}
