using System;
using System.Net;
using System.Net.Sockets;
using ArenaXunit;
using ArenaXunit.Dep;
using ArenaXunit.Xunit;
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

    internal ArenaLifecycleComponentTest(Fixture fixture)
    {
        _arena = fixture.Arena;
    }

    internal class Fixture : ArenaCollectionFixture<EmptyMatchTopology>
    {
    }

    internal class EmptyMatchTopology : IArenaTopology
    {
        public Match Configure() => new MatchBuilder("lifecycle-empty-match").Build();
    }

    [Fact]
    public void openArena_withEmptyMatch_opensAndClosesSuccessfully()
    {
        Assert.NotNull(_arena);
    }

    [Fact]
    public void openArena_getPlaybook_withNoPlaybooksRegistered_returnsNull()
    {
        var playbook = _arena.GetPlaybook(typeof(object));
        Assert.Null(playbook);
    }

    [Fact]
    public void openArena_methodsAfterDispose_throwObjectDisposedException()
    {
        ((IDisposable)_arena).Dispose();
        Assert.Throws<ObjectDisposedException>(() => _arena.GetPlaybook(typeof(object)));
    }
}

public class ArenaOauthComponentTest : IClassFixture<ArenaOauthComponentTest.Fixture>
{
    private static readonly int _port = TestRuntime.AllocatePort();

    internal class Fixture : ArenaCollectionFixture<OauthMatchTopology>
    {
    }

    internal class OauthMatchTopology : IArenaTopology
    {
        public Match Configure()
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
    internal void openArena_withOauthDependency_opensAndClosesSuccessfully(Fixture fixture)
    {
        Assert.NotNull(fixture.Arena);
    }
}

public class ArenaDisposeComponentTest : IClassFixture<ArenaDisposeComponentTest.Fixture>
{
    internal class Fixture : ArenaCollectionFixture<DisposeMatchTopology>
    {
    }

    internal class DisposeMatchTopology : IArenaTopology
    {
        public Match Configure() => new MatchBuilder("lifecycle-dispose-match").Build();
    }

    [Fact]
    internal void openArena_dispose_canBeCalledMultipleTimes(Fixture fixture)
    {
        var arena = fixture.Arena;
        ((IDisposable)arena).Dispose();
        ((IDisposable)arena).Dispose();
    }
}

public class ArenaCollectionSharingComponentTest
{
    private static int _sharedTopologyOpenCount = 0;

    internal class SharedTopologyConfig : IArenaTopology
    {
        public Match Configure()
        {
            _sharedTopologyOpenCount++;
            return new MatchBuilder("shared-lifecycle-match").Build();
        }
    }

    internal class ConcreteFixture : ArenaCollectionFixture<SharedTopologyConfig>
    {
    }

    [Fact]
    public void collectionFixture_opensArenaOnceForSharedTopology()
    {
        _sharedTopologyOpenCount = 0;

        var fixture1 = new ConcreteFixture();
        Assert.Equal(1, _sharedTopologyOpenCount);
        Assert.NotNull(fixture1.Arena);

        var fixture2 = new ConcreteFixture();
        Assert.Equal(1, _sharedTopologyOpenCount);
        Assert.Same(fixture1.Arena, fixture2.Arena);

        fixture1.Dispose();
        Assert.NotNull(fixture2.Arena);

        fixture2.Dispose();
    }
}
