using System;
using System.Collections.Concurrent;
using System.Runtime.InteropServices;
using Microsoft.Extensions.Logging;

namespace ArenaDotnet.Xunit.Ffi;

[UnmanagedFunctionPointer(CallingConvention.Cdecl)]
internal delegate void ArenaLogCallback(
    int level,
    IntPtr targetUtf8,
    long tsNanos,
    IntPtr messageUtf8,
    IntPtr callerFileUtf8,
    int callerLine,
    IntPtr userData);

internal static class ArenaLogTarget
{
    private static readonly ConcurrentDictionary<ulong, LogEntry> Entries = new();

    public static ulong RegisterForLogger(ILogger logger)
    {
        return Register(new ArenaLogRouting(logger));
    }

    public static ulong RegisterForLoggerFactory(ILoggerFactory loggerFactory)
    {
        return Register(new ArenaLogRouting(loggerFactory));
    }

    private static ulong Register(ArenaLogRouting context)
    {
        var userDataHandle = GCHandle.Alloc(context);
        var callback = new ArenaLogCallback(Invoke);
        var token = ArenaNativeLib.arena_add_log_target(callback, GCHandle.ToIntPtr(userDataHandle));
        if (token == 0)
        {
            userDataHandle.Free();
            throw new ArenaBindingError("arena_add_log_target failed");
        }
        Entries[token] = new LogEntry(callback, userDataHandle);
        return token;
    }

    public static void Unregister(ulong token)
    {
        if (!Entries.TryRemove(token, out var entry))
            return;
        ArenaNativeLib.arena_remove_log_target(token);
        entry.UserDataHandle.Free();
        GC.KeepAlive(entry.Callback);
    }

    private static void Invoke(
        int level, IntPtr targetUtf8, long tsNanos, IntPtr messageUtf8,
        IntPtr callerFileUtf8, int callerLine, IntPtr userData)
    {
        try
        {
            var gcHandle = GCHandle.FromIntPtr(userData);
            var routing = (ArenaLogRouting)gcHandle.Target!;
            var logLevel = MapLogLevel(level);
            var loggerName = LoggerNameOf(targetUtf8);
            var logger = routing.LoggerFor(loggerName);
            var message = routing.MessageFor(loggerName, ArenaNativeStrings.FromUtf8Ptr(messageUtf8));
            logger.Log(logLevel, 0, message, null, (s, e) => message);
        }
        catch (Exception ex)
        {
            Console.Error.WriteLine($"ArenaLogTarget: logger threw while handling a native log callback: {ex}");
        }
    }

    private static string LoggerNameOf(IntPtr targetUtf8)
    {
        return targetUtf8 == IntPtr.Zero ? string.Empty : ArenaNativeStrings.FromUtf8Ptr(targetUtf8);
    }

    private static LogLevel MapLogLevel(int level)
    {
        return level switch
        {
            1 => LogLevel.Error,
            2 => LogLevel.Warning,
            3 => LogLevel.Information,
            4 => LogLevel.Debug,
            5 => LogLevel.Trace,
            _ => LogLevel.Information
        };
    }

    private sealed class LogEntry
    {
        public LogEntry(ArenaLogCallback callback, GCHandle userDataHandle)
        {
            Callback = callback;
            UserDataHandle = userDataHandle;
        }

        public ArenaLogCallback Callback { get; }
        public GCHandle UserDataHandle { get; }
    }
}
