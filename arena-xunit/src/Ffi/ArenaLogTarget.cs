using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;
using Microsoft.Extensions.Logging;

namespace ArenaXunit.Ffi;

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
    private static readonly object Lock = new object();

    private static string PtrToStringUTF8(IntPtr ptr)
    {
        if (ptr == IntPtr.Zero)
            return "";
        int len = 0;
        while (Marshal.ReadByte(ptr, len) != 0)
            len++;
        byte[] buffer = new byte[len];
        Marshal.Copy(ptr, buffer, 0, len);
        return Encoding.UTF8.GetString(buffer);
    }
    private static readonly Dictionary<ulong, LogEntry> Entries = new Dictionary<ulong, LogEntry>();

    public static ulong RegisterForLogger(ILogger logger, ArenaLogLevel level)
    {
        var context = new LogContext(logger);
        var userDataHandle = GCHandle.Alloc(context);
        var callback = new ArenaLogCallback(Invoke);
        var token = ArenaNativeLib.arena_add_log_target(callback, GCHandle.ToIntPtr(userDataHandle));
        if (token == 0)
        {
            userDataHandle.Free();
            throw new ArenaBindingError("arena_add_log_target failed");
        }
        lock (Lock)
        {
            Entries[token] = new LogEntry(callback, userDataHandle);
        }
        return token;
    }

    public static void Unregister(ulong token)
    {
        LogEntry entry;
        lock (Lock)
        {
            if (!Entries.TryGetValue(token, out entry))
                return;
            Entries.Remove(token);
        }
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
            var context = (LogContext)gcHandle.Target!;
            var logLevel = MapLogLevel(level);
            var message = PtrToStringUTF8(messageUtf8) ?? string.Empty;
            context.Logger.Log(logLevel, 0, message, null, (s, e) => message);
        }
        catch
        {
        }
    }

    private static LogLevel MapLogLevel(int level)
    {
        return level switch
        {
            0 => LogLevel.Trace,
            1 => LogLevel.Debug,
            2 => LogLevel.Information,
            3 => LogLevel.Warning,
            4 => LogLevel.Error,
            5 => LogLevel.Critical,
            _ => LogLevel.Information
        };
    }

    private sealed class LogContext
    {
        public ILogger Logger { get; }
        public LogContext(ILogger logger) => Logger = logger;
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
