using System;
using System.Collections.Generic;
using System.Threading;

namespace ArenaDotnet.Xunit;

internal static class ArenaShutdown
{
    private static readonly object Gate = new();
    private static readonly List<WeakReference<OpenArena>> Open = new();
    private static int _hooksRegistered;

    internal static void EnsureHooksRegistered() => RegisterHooksOnce();

    internal static void Track(OpenArena arena)
    {
        RegisterHooksOnce();
        lock (Gate)
        {
            Open.RemoveAll(static reference => !reference.TryGetTarget(out _));
            Open.Add(new WeakReference<OpenArena>(arena));
        }
    }

    internal static void Untrack(OpenArena arena)
    {
        lock (Gate)
        {
            Open.RemoveAll(reference =>
                !reference.TryGetTarget(out var tracked) || ReferenceEquals(tracked, arena));
        }
    }

    private static void RegisterHooksOnce()
    {
        if (Interlocked.CompareExchange(ref _hooksRegistered, 1, 0) != 0)
            return;

        AppDomain.CurrentDomain.ProcessExit += (_, _) => CloseAll();
        Console.CancelKeyPress += (_, _) => CloseAll();
    }

    private static void CloseAll()
    {
        List<WeakReference<OpenArena>> pending;
        lock (Gate)
        {
            pending = new List<WeakReference<OpenArena>>(Open);
            Open.Clear();
        }

        foreach (var reference in pending)
        {
            if (!reference.TryGetTarget(out var arena))
                continue;

            try
            {
                arena.Dispose();
            }
            catch (Exception ex)
            {
                Console.Error.WriteLine($"arena: teardown on shutdown failed: {ex.Message}");
            }
        }
    }
}
