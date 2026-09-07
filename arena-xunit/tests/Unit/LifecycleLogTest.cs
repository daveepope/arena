using System;
using System.Collections.Generic;
using ArenaDotnet.Xunit.Ffi;
using ArenaDotnet.Xunit.Lifecycle;
using Microsoft.Extensions.Logging;
using Xunit;

namespace ArenaDotnet.Xunit.UnitTest;

public class LifecycleLogTest
{
    private sealed record CapturedLine(string LoggerName, LogLevel Level, string Message);

    private sealed class ListLoggerFactory : ILoggerFactory
    {
        public readonly List<CapturedLine> Lines = new();

        public ILogger CreateLogger(string categoryName) => new ListLogger(categoryName, Lines);

        public void AddProvider(ILoggerProvider provider)
        {
        }

        public void Dispose()
        {
        }
    }

    private sealed class ListLogger : ILogger
    {
        private readonly string _name;
        private readonly List<CapturedLine> _lines;

        public ListLogger(string name, List<CapturedLine> lines)
        {
            _name = name;
            _lines = lines;
        }

        public IDisposable? BeginScope<TState>(TState state) where TState : notnull => null;

        public bool IsEnabled(LogLevel level) => true;

        public void Log<TState>(LogLevel level, EventId eventId, TState state,
            Exception? exception, Func<TState, Exception?, string> formatter)
        {
            _lines.Add(new CapturedLine(_name, level, formatter(state, exception)));
        }
    }

    private static (ArenaLogRouting Routing, List<CapturedLine> Lines) FactoryRouting()
    {
        var factory = new ListLoggerFactory();
        return (new ArenaLogRouting(factory), factory.Lines);
    }

    [Theory]
    [InlineData("orders", "arena.orders")]
    [InlineData("orders.v2", "arena.orders_v2")]
    [InlineData("  ", "arena")]
    [InlineData("", "arena")]
    [InlineData("\u00A0\u2007", "arena")]
    public void ArenaLoggerName_Identifier_MatchesDispatcherNamespace(string arenaId, string expected)
    {
        Assert.Equal(expected, LifecycleLog.ArenaLoggerName(arenaId));
    }

    [Fact]
    public void LogTransition_CleanState_LogsInfoUnderArenaLogger()
    {
        var (routing, lines) = FactoryRouting();
        var state = ArenaState.Parse(
            "{\"id\":\"transition-clean\",\"state\":\"dependencies_starting\",\"at\":\"t\"}");

        LifecycleLog.LogTransition(state, routing);

        var line = Assert.Single(lines);
        Assert.Equal("arena.transition-clean", line.LoggerName);
        Assert.Equal(LogLevel.Information, line.Level);
        Assert.Equal("dependencies_starting", line.Message);
    }

    [Fact]
    public void LogTransition_FaultedState_LogsErrorWithFaultCount()
    {
        var (routing, lines) = FactoryRouting();
        var state = ArenaState.Parse(ArenaStateTest.FixtureStateJson);

        LifecycleLog.LogTransition(state, routing);

        var line = Assert.Single(lines);
        Assert.Equal("arena.orders", line.LoggerName);
        Assert.Equal(LogLevel.Error, line.Level);
        Assert.Equal("arena_faulted | faults=1", line.Message);
    }

    [Fact]
    public void LogClosingSummary_State_LogsTokenAndFaultCount()
    {
        var (routing, lines) = FactoryRouting();
        var state = ArenaState.Parse(
            "{\"id\":\"closing-summary\",\"state\":\"arena_closed\",\"at\":\"t\"}");

        LifecycleLog.LogClosingSummary(state, routing);

        var line = Assert.Single(lines);
        Assert.Equal("arena.closing-summary", line.LoggerName);
        Assert.Equal(LogLevel.Information, line.Level);
        Assert.Equal("closing summary | state=arena_closed | faults=0", line.Message);
    }

    [Fact]
    public void LogTransitionDocument_MatchingArenaId_LogsTheTransition()
    {
        var (routing, lines) = FactoryRouting();

        LifecycleLog.LogTransitionDocument(
            "doc-valid", routing, "{\"id\":\"doc-valid\",\"state\":\"arena_open\",\"at\":\"t\"}");

        var line = Assert.Single(lines);
        Assert.Equal("arena_open", line.Message);
    }

    [Fact]
    public void LogTransitionDocument_OtherArenaId_LogsNothing()
    {
        var (routing, lines) = FactoryRouting();

        LifecycleLog.LogTransitionDocument(
            "this-arena", routing, "{\"id\":\"other-arena\",\"state\":\"arena_open\",\"at\":\"t\"}");

        Assert.Empty(lines);
    }

    [Fact]
    public void LogTransitionDocument_UnparseableDocument_WarnsOnRootLogger()
    {
        var (routing, lines) = FactoryRouting();

        LifecycleLog.LogTransitionDocument("any", routing, "{broken");

        var line = Assert.Single(lines);
        Assert.Equal("arena", line.LoggerName);
        Assert.Equal(LogLevel.Warning, line.Level);
        Assert.Equal("unparseable arena state transition: {broken", line.Message);
    }

    [Fact]
    public void LogClosingSummaryDocument_ValidDocument_LogsTheSummary()
    {
        var (routing, lines) = FactoryRouting();

        LifecycleLog.LogClosingSummaryDocument(
            "{\"id\":\"close-doc-valid\",\"state\":\"arena_closed\",\"at\":\"t\"}", routing);

        var line = Assert.Single(lines);
        Assert.Equal("closing summary | state=arena_closed | faults=0", line.Message);
    }

    [Fact]
    public void LogClosingSummaryDocument_UnparseableDocument_WarnsOnRootLogger()
    {
        var (routing, lines) = FactoryRouting();

        LifecycleLog.LogClosingSummaryDocument("{broken", routing);

        var line = Assert.Single(lines);
        Assert.Equal("arena", line.LoggerName);
        Assert.Equal(LogLevel.Warning, line.Level);
        Assert.Equal("unparseable arena closing state: {broken", line.Message);
    }

    [Fact]
    public void LogTransition_SingleLoggerRouting_PrefixesLoggerNameIntoMessage()
    {
        var lines = new List<CapturedLine>();
        var routing = new ArenaLogRouting(new ListLogger("plain", lines));
        var state = ArenaState.Parse("{\"id\":\"orders\",\"state\":\"arena_open\",\"at\":\"t\"}");

        LifecycleLog.LogTransition(state, routing);

        var line = Assert.Single(lines);
        Assert.Equal("arena.orders | arena_open", line.Message);
    }
}
