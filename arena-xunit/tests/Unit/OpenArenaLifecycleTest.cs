using System;
using System.Collections.Generic;
using System.Threading.Tasks;
using ArenaDotnet.Xunit.Component;
using ArenaDotnet.Xunit.Ffi;
using ArenaDotnet.Xunit.Lifecycle;
using Microsoft.Extensions.Logging;
using Xunit;

namespace ArenaDotnet.Xunit.UnitTest;

public class OpenArenaLifecycleTest
{
    private sealed class TransitionCapture : ILoggerFactory
    {
        public readonly List<(string LoggerName, LogLevel Level, string Message)> Lines = new();

        public ILogger CreateLogger(string categoryName) => new CaptureLogger(categoryName, this);

        public void AddProvider(ILoggerProvider provider)
        {
        }

        public void Dispose()
        {
        }

        private sealed class CaptureLogger : ILogger
        {
            private readonly string _name;
            private readonly TransitionCapture _owner;

            public CaptureLogger(string name, TransitionCapture owner)
            {
                _name = name;
                _owner = owner;
            }

            public IDisposable? BeginScope<TState>(TState state) where TState : notnull => null;

            public bool IsEnabled(LogLevel level) => true;

            public void Log<TState>(LogLevel level, EventId eventId, TState state,
                Exception? exception, Func<TState, Exception?, string> formatter)
            {
                lock (_owner.Lines)
                {
                    _owner.Lines.Add((_name, level, formatter(state, exception)));
                }
            }
        }
    }

    private static ClosedArena ClosedArenaWithMissingBinary(string arenaName)
    {
        var component = new ExecutableComponentBuilder("xunit-lifecycle-missing-binary")
            .WithExecutablePath("xunit-lifecycle-probe-does-not-exist")
            .Build();
        var match = new MatchBuilder(arenaName + "-match").AddComponent(component).Build();
        return new ClosedArena(arenaName, match);
    }

    [Fact]
    public async Task OpenAsync_ComponentFailsToStart_ThrowsLifecycleErrorCarryingState()
    {
        var closed = ClosedArenaWithMissingBinary("xunit-lifecycle-faulted");

        var error = await Assert.ThrowsAsync<ArenaLifecycleError>(() => closed.OpenAsync());

        Assert.Contains("is arena_faulted", error.Message);
        Assert.NotNull(error.State);
        Assert.True(error.State!.IsFaulted());
        Assert.Equal("xunit-lifecycle-faulted", error.State!.Id);
        Assert.NotEmpty(error.State!.Components);
        Assert.DoesNotContain("panicked at", error.Message);
    }

    [Fact]
    public async Task State_OpenArena_ReturnsOpenStateWithArenaId()
    {
        var closed = new ClosedArena(
            "xunit-state-accessor", new MatchBuilder("xunit-state-accessor-match").Build());
        var arena = await closed.OpenAsync();
        try
        {
            var state = arena.State();
            Assert.Equal("xunit-state-accessor", state.Id);
            Assert.Equal("arena_open", state.State);
        }
        finally
        {
            arena.Dispose();
        }
    }

    [Fact]
    public async Task State_AfterDispose_ThrowsObjectDisposedException()
    {
        var closed = new ClosedArena(
            "xunit-state-disposed", new MatchBuilder("xunit-state-disposed-match").Build());
        var arena = await closed.OpenAsync();
        arena.Dispose();

        Assert.Throws<ObjectDisposedException>(() => arena.State());
    }

    [Fact]
    public async Task Dispose_EmptyMatchWithLoggerFactory_LogsTransitionsInOrderThenClosingSummary()
    {
        var capture = new TransitionCapture();
        var closed = new ClosedArena(
            "xunit-transition-stream",
            new MatchBuilder("xunit-transition-stream-match").Build(),
            ArenaLogLevel.Info,
            capture);

        (await closed.OpenAsync()).Dispose();

        var messages = capture.Lines
            .FindAll(l => l.LoggerName == "arena.xunit-transition-stream")
            .ConvertAll(l => l.Message);
        Assert.Contains("arena_starting", messages);
        Assert.Contains("arena_open", messages);
        Assert.Contains("arena_closing", messages);
        Assert.Contains("arena_closed", messages);
        Assert.True(messages.IndexOf("arena_starting") < messages.IndexOf("arena_open"));
        Assert.True(messages.IndexOf("arena_open") < messages.IndexOf("arena_closing"));
        Assert.True(messages.IndexOf("arena_closing") < messages.IndexOf("arena_closed"));
        Assert.Equal("closing summary | state=arena_closed | faults=0", messages[messages.Count - 1]);
    }

    [Fact]
    public async Task Unregister_RemovedObserver_StopsReceivingTransitions()
    {
        var documents = new List<string>();
        var token = ArenaLifecycleObservers.Register(document =>
        {
            lock (documents)
            {
                documents.Add(document);
            }
        });
        try
        {
            var first = new ClosedArena(
                "xunit-observer-live", new MatchBuilder("xunit-observer-live-match").Build());
            (await first.OpenAsync()).Dispose();
            lock (documents)
            {
                Assert.Contains(documents, d => d.Contains("xunit-observer-live"));
            }
        }
        finally
        {
            ArenaLifecycleObservers.Unregister(token);
        }

        var second = new ClosedArena(
            "xunit-observer-removed", new MatchBuilder("xunit-observer-removed-match").Build());
        (await second.OpenAsync()).Dispose();

        lock (documents)
        {
            Assert.DoesNotContain(documents, d => d.Contains("xunit-observer-removed"));
        }
    }

}
