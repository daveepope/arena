using System;
using ArenaDotnet.Xunit.Ffi;
using Microsoft.Extensions.Logging;

namespace ArenaDotnet.Xunit.Lifecycle;

public static class LifecycleLog
{
    public static string ArenaLoggerName(string arenaId)
    {
        var segment = (arenaId ?? string.Empty).Trim().Replace('.', '_');
        if (segment.Length == 0)
            return ArenaLogRouting.RootLoggerName;
        return $"{ArenaLogRouting.RootLoggerName}.{segment}";
    }

    internal static void LogTransitionDocument(string arenaId, ArenaLogRouting routing, string document)
    {
        ArenaState state;
        try
        {
            state = ArenaState.Parse(document);
        }
        catch (ArgumentException)
        {
            Emit(routing, ArenaLogRouting.RootLoggerName, LogLevel.Warning,
                $"unparseable arena state transition: {document}");
            return;
        }
        if (!string.Equals(state.Id, arenaId, StringComparison.Ordinal))
            return;
        LogTransition(state, routing);
    }

    internal static void LogTransition(ArenaState state, ArenaLogRouting routing)
    {
        var line = state.Faults.Count > 0 ? $"{state.State} | faults={state.Faults.Count}" : state.State;
        Emit(routing, ArenaLoggerName(state.Id),
            state.IsFaulted() ? LogLevel.Error : LogLevel.Information, line);
    }

    internal static void LogClosingSummaryDocument(string document, ArenaLogRouting routing)
    {
        ArenaState state;
        try
        {
            state = ArenaState.Parse(document);
        }
        catch (ArgumentException)
        {
            Emit(routing, ArenaLogRouting.RootLoggerName, LogLevel.Warning,
                $"unparseable arena closing state: {document}");
            return;
        }
        LogClosingSummary(state, routing);
    }

    internal static void LogClosingSummary(ArenaState state, ArenaLogRouting routing)
    {
        Emit(routing, ArenaLoggerName(state.Id), LogLevel.Information,
            $"closing summary | state={state.State} | faults={state.Faults.Count}");
    }

    private static void Emit(ArenaLogRouting routing, string loggerName, LogLevel level, string line)
    {
        var logger = routing.LoggerFor(loggerName);
        var message = routing.MessageFor(loggerName, line);
        logger.Log(level, 0, message, null, static (s, _) => s);
    }
}
