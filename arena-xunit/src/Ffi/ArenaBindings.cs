using System;
using System.Runtime.InteropServices;
using System.Text;

namespace ArenaXunit.Ffi;

internal static class ArenaBindings
{
    private static string PtrToStringUTF8(IntPtr ptr)
    {
        if (ptr == IntPtr.Zero)
            return "";

        int len = 0;
        IntPtr current = ptr;
        while (Marshal.ReadByte(current, len) != 0)
            len++;

        byte[] buffer = new byte[len];
        Marshal.Copy(ptr, buffer, 0, len);
        return Encoding.UTF8.GetString(buffer);
    }

    internal static IntPtr OpenArena(string name, string configJson, ArenaLogLevel level)
    {
        ArenaNativeLib.arena_set_log_level((int)level);
        IntPtr handle = ArenaNativeLib.arena_open(name, configJson, out var errOut);
        if (handle == IntPtr.Zero)
            throw TakeErr(errOut, "arena_open failed");
        return handle;
    }

    internal static void CloseArena(IntPtr handle)
    {
        ArenaNativeLib.arena_close(handle);
    }

    internal static void SoftReset(IntPtr handle, string dependencyIdentifier)
    {
        var result = ArenaNativeLib.arena_soft_reset(handle, dependencyIdentifier, out var errOut);
        if (result != 0)
            throw TakeErr(errOut, "arena_soft_reset failed");
    }

    internal static void HardReset(IntPtr handle, string dependencyIdentifier)
    {
        var result = ArenaNativeLib.arena_hard_reset(handle, dependencyIdentifier, out var errOut);
        if (result != 0)
            throw TakeErr(errOut, "arena_hard_reset failed");
    }

    internal static void SetDispatcherDependencyAllowJson(string? json)
    {
        ArenaNativeLib.arena_dispatcher_dependency_allow_json_set(json);
    }

    internal static void SetDispatcherComponentAllowJson(string? json)
    {
        ArenaNativeLib.arena_dispatcher_component_allow_json_set(json);
    }

    internal static Playbook.ActivePlaybook MatchPlaybookRun(IntPtr arena, string identifier)
    {
        var handle = ArenaNativeLib.arena_match_playbook_run(arena, identifier, out var errOut);
        if (handle == IntPtr.Zero)
            throw TakeErr(errOut, "arena_match_playbook_run failed");
        return new Playbook.ActivePlaybook(handle);
    }

    internal static void ActivePlaybookDrop(IntPtr handle)
    {
        var result = ArenaNativeLib.arena_active_playbook_drop(handle, out var errOut);
        if (result != 0)
            throw TakeErr(errOut, "arena_active_playbook_drop failed");
    }

    internal static IntPtr HttpPlaybookOpen(IntPtr arena, string specJson)
    {
        var handle = ArenaNativeLib.arena_http_playbook_open(arena, specJson, out var errOut);
        if (handle == IntPtr.Zero)
            throw TakeErr(errOut, "arena_http_playbook_open failed");
        return handle;
    }

    internal static void HttpPlaybookVerify(IntPtr handle, string specJson)
    {
        var result = ArenaNativeLib.arena_http_playbook_verify(handle, specJson, out var errOut);
        if (result != 0)
            throw TakeErr(errOut, "arena_http_playbook_verify failed");
    }

    internal static void MssqlPlaybookVerify(IntPtr handle, string specJson)
    {
        var result = ArenaNativeLib.arena_mssql_playbook_verify(handle, specJson, out var errOut);
        if (result != 0)
            throw TakeErr(errOut, "arena_mssql_playbook_verify failed");
    }

    internal static string OauthLoopbackTlsPemJson()
    {
        var ptr = ArenaNativeLib.arena_oauth_loopback_tls_pem_json(out var errOut);
        if (ptr == IntPtr.Zero)
            throw TakeErr(errOut, "arena_oauth_loopback_tls_pem_json failed");
        var json = PtrToStringUTF8(ptr) ?? string.Empty;
        ArenaNativeLib.arena_free_string(ptr);
        return json;
    }

    private static ArenaBindingError TakeErr(IntPtr errOut, string operation)
    {
        if (errOut != IntPtr.Zero)
        {
            var message = PtrToStringUTF8(errOut) ?? operation;
            ArenaNativeLib.arena_free_string(errOut);
            return new ArenaBindingError(message);
        }
        return new ArenaBindingError(operation);
    }
}
