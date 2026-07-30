using System;
using System.Net;
using System.Net.Sockets;
using ArenaXunit;
using ArenaXunit.Dep;
using ArenaXunit.Topology;
using Xunit;

namespace ArenaXunit.ComponentTest;

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

public class ArenaLifecycleComponentTest : IClassFixture<ArenaLifecycleComponentTest.Fixture>
{
    private readonly OpenArena _arena;

    public ArenaLifecycleComponentTest(Fixture fixture)
    {
        _arena = fixture.Arena;
    }

    public class Fixture : ArenaCollectionFixture<EmptyMatchTopology>
    {
    }

    public class EmptyMatchTopology
    {
    }

    [Fact]
    public void OpenArena_WithEmptyMatch_OpensAndClosesSuccessfully()
    {
        Assert.NotNull(_arena);
    }

    [Fact]
    public void OpenArena_GetPlaybook_WithNoPlaybooksRegistered_ReturnsNull()
    {
        var playbook = _arena.GetPlaybook(typeof(object));
        Assert.Null(playbook);
    }

    [Fact]
    public void OpenArena_MethodsAfterDispose_ThrowObjectDisposedException()
    {
        var closedArena = new ClosedArena("dispose-test", new MatchBuilder("dispose-test-match").Build());
        var openArena = closedArena.OpenAsync().Result;
        ((IDisposable)openArena).Dispose();
        Assert.Throws<ObjectDisposedException>(() => openArena.GetPlaybook(typeof(object)));
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

    public class Fixture : ArenaCollectionFixture<OauthMatchTopology>
    {
    }

    public class OauthMatchTopology
    {
        [ArenaDependency]
        public static readonly OauthDependency Oauth = new OauthDependencyBuilder("test-oauth")
            .WithPort(_port)
            .WithListenIp("0.0.0.0")
            .Build();
    }

    [Fact]
    internal void OpenArena_WithOauthDependency_OpensAndClosesSuccessfully()
    {
        Assert.NotNull(_fixture.Arena);
    }
}

public class ArenaDisposeComponentTest : IClassFixture<ArenaDisposeComponentTest.Fixture>
{
    private readonly Fixture _fixture;

    public ArenaDisposeComponentTest(Fixture fixture)
    {
        _fixture = fixture;
    }

    public class Fixture : ArenaCollectionFixture<DisposeMatchTopology>
    {
    }

    public class DisposeMatchTopology
    {
    }

    [Fact]
    internal void OpenArena_Dispose_CanBeCalledMultipleTimes()
    {
        var arena = _fixture.Arena;
        ((IDisposable)arena).Dispose();
        ((IDisposable)arena).Dispose();
    }
}

public class ArenaCollectionSharingComponentTest
{
    internal class SharedTopologyConfig
    {
    }

    internal class ConcreteFixture : ArenaCollectionFixture<SharedTopologyConfig>
    {
    }

    [Fact]
    public void CollectionFixture_SharesArenaForSameTopology()
    {
        var fixture1 = new ConcreteFixture();
        Assert.NotNull(fixture1.Arena);

        var fixture2 = new ConcreteFixture();
        Assert.Same(fixture1.Arena, fixture2.Arena);

        fixture1.Dispose();
        Assert.NotNull(fixture2.Arena);

        fixture2.Dispose();
    }
}
