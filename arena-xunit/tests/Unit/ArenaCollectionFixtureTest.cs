using System;
using ArenaDotnet.Xunit.Dep;
using Microsoft.Extensions.Logging;
using Xunit;

namespace ArenaDotnet.Xunit.UnitTest;

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
#pragma warning disable CS0414
        [ArenaDependency]
        private static readonly HttpDependency? NullDependency = null;
#pragma warning restore CS0414
    }

    private sealed class StubMatchPiece : IArenaDependency, IArenaComponent
    {
        public string Identifier { get; }
        public StubMatchPiece(string identifier) => Identifier = identifier;
        public string ForFfi() => $"{{\"identifier\":\"{Identifier}\"}}";
    }

    private class LogsIdentifierFixture : ArenaCollectionFixture
    {
        protected override Match Configure() => new MatchBuilder("logs-identifier-match").Build();

        [ArenaDependency(Logs = true)]
        private static readonly StubMatchPiece LoggedDependency = new StubMatchPiece("stub-dependency-id");

        [ArenaComponent(Logs = true)]
        private static readonly StubMatchPiece LoggedComponent = new StubMatchPiece("stub-component-id");
    }

    private sealed class RecordingLogger : ILogger
    {
        public bool Logged { get; private set; }
        public IDisposable? BeginScope<TState>(TState state) where TState : notnull => null;
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

    private class NoOauthDependencyFieldFixture : ArenaCollectionFixture
    {
        protected override Match Configure() => new MatchBuilder("no-oauth-dependency-match").Build();
    }

    private class MultipleOauthDependencyFieldsFixture : ArenaCollectionFixture
    {
        [ArenaDependency]
        private static readonly OauthDependency OauthOne = new OauthDependencyBuilder("oauth-one").Build();

        [ArenaDependency]
        private static readonly OauthDependency OauthTwo = new OauthDependencyBuilder("oauth-two").Build();

        protected override Match Configure() => new MatchBuilder("multiple-oauth-dependency-match").Build();
    }

    private class SingleOauthDependencyFieldFixture : ArenaCollectionFixture
    {
        [ArenaDependency]
        public static readonly OauthDependency Oauth = new OauthDependencyBuilder("oauth-single").Build();

        protected override Match Configure() => new MatchBuilder("single-oauth-dependency-match").Build();
    }

    private class BaseOauthDependencyFixture : ArenaCollectionFixture
    {
        [ArenaDependency]
        public static readonly OauthDependency Oauth = new OauthDependencyBuilder("oauth-base").Build();

        protected override Match Configure() => new MatchBuilder("inherited-oauth-dependency-match").Build();
    }

    private sealed class SubOauthDependencyFixture : BaseOauthDependencyFixture
    {
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

    [Fact]
    public void CollectLogIdentifiers_DependencyAndComponentFieldsLogsTrue_ExtractsIdentifiersFromForFfiJson()
    {
        using var fixture = new LogsIdentifierFixture();
        var (dependencyIds, componentIds) = fixture.CollectLogIdentifiers();
        Assert.Contains("stub-dependency-id", dependencyIds);
        Assert.Contains("stub-component-id", componentIds);
    }

    [Fact]
    public void GetDependency_NoArenaDependencyField_ThrowsInvalidOperationException()
    {
        using var fixture = new NoOauthDependencyFieldFixture();

        var error = Assert.Throws<InvalidOperationException>(() => fixture.GetDependency<OauthDependency>());

        Assert.Contains("found none", error.Message);
    }

    [Fact]
    public void GetDependency_MultipleArenaDependencyFieldsOfType_ThrowsInvalidOperationException()
    {
        using var fixture = new MultipleOauthDependencyFieldsFixture();

        var error = Assert.Throws<InvalidOperationException>(() => fixture.GetDependency<OauthDependency>());

        Assert.Contains("found more than one", error.Message);
    }

    [Fact]
    public void GetDependency_SingleArenaDependencyField_ReturnsThatDependency()
    {
        using var fixture = new SingleOauthDependencyFieldFixture();

        var dependency = fixture.GetDependency<OauthDependency>();

        Assert.Equal(SingleOauthDependencyFieldFixture.Oauth.Identifier, dependency.Identifier);
    }

    [Fact]
    public void GetDependency_FieldDeclaredOnBaseClass_ReturnsThatDependency()
    {
        using var fixture = new SubOauthDependencyFixture();

        var dependency = fixture.GetDependency<OauthDependency>();

        Assert.Equal(BaseOauthDependencyFixture.Oauth.Identifier, dependency.Identifier);
    }
}
