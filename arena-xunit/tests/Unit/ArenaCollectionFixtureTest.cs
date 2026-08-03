using System;
using ArenaXunit.Dep;
using Microsoft.Extensions.Logging;
using Xunit;

namespace ArenaXunit.UnitTest;

public class ArenaCollectionFixtureTest
{
    private static int _sharedMatchOpenCount = 0;

    private class ConcreteFixture : ArenaCollectionFixture
    {
        protected override Match Configure()
        {
            _sharedMatchOpenCount++;
            return new MatchBuilder("shared-lifecycle-match").Build();
        }
    }

    private class NoAttributesFixture : ArenaCollectionFixture
    {
    }

    private class NullAttributeFieldFixture : ArenaCollectionFixture
    {
        [ArenaDependency]
        private static readonly HttpDependency? NullDependency = null;
    }

    private sealed class RecordingLogger : ILogger
    {
        public bool Logged { get; private set; }
        public IDisposable? BeginScope<TState>(TState state) => null;
        public bool IsEnabled(LogLevel logLevel) => true;
        public void Log<TState>(LogLevel logLevel, EventId eventId, TState state,
            Exception? exception, Func<TState, Exception?, string> formatter)
        {
            Logged = true;
        }
    }

    private class MultipleArenaLoggerFieldsFixture : ArenaCollectionFixture
    {
        protected override Match Configure() => new MatchBuilder("multi-logger-match").Build();

        [ArenaLogger]
        private static readonly ILogger LoggerOne = new RecordingLogger();

        [ArenaLogger]
        private static readonly ILogger LoggerTwo = new RecordingLogger();
    }

    private class ArenaLoggerFixture : ArenaCollectionFixture
    {
        public static RecordingLogger Logger { get; } = new RecordingLogger();

        [ArenaLogger]
        private static readonly ILogger LoggerField = Logger;

        protected override Match Configure() => new MatchBuilder("arena-logger-match").Build();
    }

    [Fact]
    public void Constructor_TwoInstancesOfSameFixtureType_SharesArenaAndOpensOnce()
    {
        _sharedMatchOpenCount = 0;

        var fixture1 = new ConcreteFixture();
        Assert.Equal(1, _sharedMatchOpenCount);
        Assert.NotNull(fixture1.Arena);

        var fixture2 = new ConcreteFixture();
        Assert.Equal(1, _sharedMatchOpenCount);
        Assert.Same(fixture1.Arena, fixture2.Arena);

        fixture1.Dispose();
        Assert.NotNull(fixture2.Arena);

        fixture2.Dispose();
    }

    [Fact]
    public void Constructor_NoConfigureOverrideAndNoArenaAttributes_ThrowsInvalidOperationException()
    {
        Assert.Throws<InvalidOperationException>(() => new NoAttributesFixture());
    }

    [Fact]
    public void Constructor_ArenaDependencyFieldIsNull_ThrowsInvalidOperationException()
    {
        var ex = Assert.Throws<InvalidOperationException>(() => new NullAttributeFieldFixture());
        Assert.Contains("must not be null", ex.Message);
    }

    [Fact]
    public void Constructor_MultipleArenaLoggerFields_ThrowsInvalidOperationException()
    {
        Assert.Throws<InvalidOperationException>(() => new MultipleArenaLoggerFieldsFixture());
    }

    [Fact]
    public void Constructor_ArenaLoggerFieldPresent_OpensWithSuppliedLogger()
    {
        using var fixture = new ArenaLoggerFixture();
        Assert.NotNull(fixture.Arena);
    }
}
