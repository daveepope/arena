using System;
using System.Collections.Concurrent;
using System.Runtime.InteropServices;

namespace ArenaDotnet.Xunit.Ffi;

[UnmanagedFunctionPointer(CallingConvention.Cdecl)]
internal delegate void ArenaLifecycleObserverCallback(IntPtr stateJsonUtf8, IntPtr userData);

internal static class ArenaLifecycleObservers
{
    private static readonly ConcurrentDictionary<ulong, ArenaLifecycleObserverCallback> Entries = new();

    public static ulong Register(Action<string> onStateDocument)
    {
        if (onStateDocument == null)
            throw new ArenaBindingError("lifecycle observer callback is null");
        var callback = new ArenaLifecycleObserverCallback((stateJsonUtf8, _) =>
        {
            try
            {
                if (stateJsonUtf8 == IntPtr.Zero)
                    return;
                var document = ArenaNativeStrings.FromUtf8Ptr(stateJsonUtf8);
                if (!string.IsNullOrEmpty(document))
                    onStateDocument(document);
            }
            catch (Exception ex)
            {
                Console.Error.WriteLine(
                    $"ArenaLifecycleObservers: observer threw while handling a native state callback: {ex}");
            }
        });
        var token = ArenaNativeLib.arena_add_lifecycle_observer(callback, IntPtr.Zero);
        if (token == 0)
            throw new ArenaBindingError("arena_add_lifecycle_observer rejected callback");
        Entries[token] = callback;
        return token;
    }

    public static void Unregister(ulong token)
    {
        if (token == 0)
            return;
        if (!Entries.TryRemove(token, out var callback))
            return;
        ArenaNativeLib.arena_remove_lifecycle_observer(token);
        GC.KeepAlive(callback);
    }
}
