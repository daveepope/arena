using System;
using System.Net;
using System.Net.Sockets;
using ArenaDotnet.Xunit;
using ArenaDotnet.Xunit.Dep;
using ArenaDotnet.Xunit.Xunit;
using Xunit;

namespace ArenaDotnet.Xunit.ComponentTest;

public static class TestRuntime
{
    private static readonly object Lock = new object();
    private static int? _nextPort;

    public static int AllocatePort()
    {
        lock (Lock)
        {
            if (_nextPort.HasValue)
            {
                var p = _nextPort.Value;
                _nextPort = p + 1;
                return p;
            }
            var port = FindOpenPort();
            _nextPort = port + 1;
            return port;
        }
    }

    private static int FindOpenPort()
    {
        var listener = new TcpListener(IPAddress.Loopback, 0);
        listener.Start();
        var port = ((IPEndPoint)listener.LocalEndpoint).Port;
        listener.Stop();
        return port;
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
