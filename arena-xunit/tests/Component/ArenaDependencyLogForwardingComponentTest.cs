using System;
using System.Collections.Generic;
using ArenaDotnet.Xunit;
using ArenaDotnet.Xunit.Dep;
using ArenaDotnet.Xunit.Ffi;
using Microsoft.Extensions.Logging;
using Xunit;

namespace ArenaDotnet.Xunit.ComponentTest;

internal sealed class RecordedLogLogger : ILogger
{
    private readonly object _lock = new object();
    private readonly List<string> _messages = new();

    public List<string> Messages
    {
        get
        {
            lock (_lock)
            {
                return new List<string>(_messages);
            }
        }
    }

    public IDisposable? BeginScope<TState>(TState state) => null;

    public bool IsEnabled(LogLevel logLevel) => true;

    public void Log<TState>(LogLevel logLevel, EventId eventId, TState state,
        Exception? exception, Func<TState, Exception?, string> formatter)
    {
        lock (_lock)
        {
            _messages.Add(formatter(state, exception));
        }
    }
}

public class DependencyLogsEnabledFixture : ArenaCollectionFixture
{
    private static readonly int _port = TestRuntime.AllocatePort();

    internal static readonly RecordedLogLogger CapturingLogger = new();

    [ArenaDependency(Logs = true)]
    private static readonly OauthDependency Oauth =
        new OauthDependencyBuilder("dependency-logs-enabled-oauth")
            .WithPort(_port)
            .WithListenIp("0.0.0.0")
            .Build();

    [ArenaLogger(Level = ArenaLogLevel.Debug)]
    private static readonly ILogger LoggerField = CapturingLogger;
}

public class DependencyLogsDisabledFixture : ArenaCollectionFixture
{
    private static readonly int _port = TestRuntime.AllocatePort();

    internal static readonly RecordedLogLogger CapturingLogger = new();

    [ArenaDependency]
    private static readonly OauthDependency Oauth =
        new OauthDependencyBuilder("dependency-logs-disabled-oauth")
            .WithPort(_port)
            .WithListenIp("0.0.0.0")
            .Build();

    [ArenaLogger(Level = ArenaLogLevel.Debug)]
    private static readonly ILogger LoggerField = CapturingLogger;
}

public class DependencyLogsEnabledComponentTest : IClassFixture<DependencyLogsEnabledFixture>
{
    private readonly DependencyLogsEnabledFixture _fixture;

    public DependencyLogsEnabledComponentTest(DependencyLogsEnabledFixture fixture)
    {
        _fixture = fixture;
    }

    [Fact]
    internal void OpenArena_DependencyLogsEnabled_ForwardsDependencyTaggedDebugLog()
    {
        Assert.NotNull(_fixture.Arena);
        Assert.Contains(
            DependencyLogsEnabledFixture.CapturingLogger.Messages,
            m => m.Contains("dependency-logs-enabled-oauth") && m.Contains("starting"));
    }
}

public class DependencyLogsDisabledComponentTest : IClassFixture<DependencyLogsDisabledFixture>
{
    private readonly DependencyLogsDisabledFixture _fixture;

    public DependencyLogsDisabledComponentTest(DependencyLogsDisabledFixture fixture)
    {
        _fixture = fixture;
    }

    [Fact]
    internal void OpenArena_DependencyLogsDisabled_DoesNotForwardDependencyTaggedDebugLog()
    {
        Assert.NotNull(_fixture.Arena);
        Assert.DoesNotContain(
            DependencyLogsDisabledFixture.CapturingLogger.Messages,
            m => m.Contains("dependency-logs-disabled-oauth") && m.Contains("starting"));
    }
}
