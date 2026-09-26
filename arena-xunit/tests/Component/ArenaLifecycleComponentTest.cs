using System;
using ArenaDotnet.Xunit;
using ArenaDotnet.Xunit.Dep;
using ArenaDotnet.Xunit.Ffi;
using ArenaDotnet.Xunit.Xunit;
using Xunit;

namespace ArenaDotnet.Xunit.ComponentTest;

public static class TestRuntime
{
    private const int EphemeralPortRangeStart = 20900;
    private const int EphemeralPortRangeEnd = 21100;

    public static int AllocatePort()
    {
        return ArenaHost.FindAvailablePort(EphemeralPortRangeStart, EphemeralPortRangeEnd, PortSearchStrategy.Random);
    }
}

public class ArenaOauthComponentTest : IClassFixture<ArenaOauthComponentTest.Fixture>
{
    private static readonly int _port = TestRuntime.AllocatePort();

    private readonly Fixture _fixture;

    public ArenaOauthComponentTest(Fixture fixture)
    {
        _fixture = fixture;
    }

    public class Fixture : ArenaCollectionFixture
    {
        protected override Match Configure()
        {
            return new MatchBuilder("lifecycle-oauth-match")
                .AddDependency(new OauthDependencyBuilder("test-oauth")
                    .WithPort(_port)
                    .WithListenIp("0.0.0.0")
                    .Build())
                .Build();
        }
    }

    [Fact]
    internal void OpenArena_WithOauthDependency_OpensAndClosesSuccessfully()
    {
        Assert.NotNull(_fixture.Arena);
    }
}
