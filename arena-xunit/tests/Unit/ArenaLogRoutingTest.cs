using System.Collections.Generic;
using ArenaDotnet.Xunit.Ffi;
using Microsoft.Extensions.Logging;
using Xunit;

namespace ArenaDotnet.Xunit.UnitTest;

public class ArenaLogRoutingTest
{
    private sealed class RecordingLoggerFactory : ILoggerFactory
    {
        public readonly List<string> Requested = new();

        public ILogger CreateLogger(string categoryName)
        {
            Requested.Add(categoryName);
            return Microsoft.Extensions.Logging.Abstractions.NullLogger.Instance;
        }

        public void AddProvider(ILoggerProvider provider) { }
        public void Dispose() { }
    }

    [Theory]
    [InlineData("arena.orders", "arena.orders")]
    [InlineData("arena.orders.dependency.orders-postgres", "arena.orders.dependency.orders-postgres")]
    [InlineData("", "arena")]
    public void LoggerFor_RecordTarget_ResolvesThroughTheFactory(string target, string expectedName)
    {
        var loggerFactory = new RecordingLoggerFactory();
        var routing = new ArenaLogRouting(loggerFactory);

        routing.LoggerFor(target);

        Assert.Equal(new List<string> { expectedName }, loggerFactory.Requested);
    }

    [Fact]
    public void LoggerFor_RepeatedTarget_ReusesTheCachedLogger()
    {
        var loggerFactory = new RecordingLoggerFactory();
        var routing = new ArenaLogRouting(loggerFactory);

        var first = routing.LoggerFor("arena.orders");
        var second = routing.LoggerFor("arena.orders");

        Assert.Same(first, second);
        Assert.Single(loggerFactory.Requested);
    }

    [Fact]
    public void LoggerFor_BareLogger_ReturnsThatLogger()
    {
        var logger = Microsoft.Extensions.Logging.Abstractions.NullLogger.Instance;
        var routing = new ArenaLogRouting(logger);

        Assert.Same(logger, routing.LoggerFor("arena.orders"));
    }

    [Fact]
    public void MessageFor_BareLogger_PrefixesTheLoggerName()
    {
        var routing = new ArenaLogRouting(Microsoft.Extensions.Logging.Abstractions.NullLogger.Instance);

        Assert.Equal("arena.orders  started", routing.MessageFor("arena.orders", "started"));
    }

    [Fact]
    public void MessageFor_LoggerFactory_LeavesTheMessageUnchanged()
    {
        var routing = new ArenaLogRouting(new RecordingLoggerFactory());

        Assert.Equal("started", routing.MessageFor("arena.orders", "started"));
    }

    [Fact]
    public void MessageFor_BlankLoggerName_LeavesTheMessageUnchanged()
    {
        var routing = new ArenaLogRouting(Microsoft.Extensions.Logging.Abstractions.NullLogger.Instance);

        Assert.Equal("started", routing.MessageFor("", "started"));
    }
}
