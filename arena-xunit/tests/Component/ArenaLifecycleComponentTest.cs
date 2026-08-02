using System;
using System.Net;
using System.Net.Sockets;
using ArenaXunit;
using ArenaXunit.Dep;
using ArenaXunit.Xunit;
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

    public class Fixture : ArenaCollectionFixture
    {
        protected override Match Configure() => new MatchBuilder("lifecycle-empty-match").Build();
    }

    [Fact]
    public void OpenArena_WithEmptyMatch_OpensAndClosesSuccessfully()
    {
        Assert.NotNull(_arena);
    }

    [Fact]
    public void OpenArena_GetPlaybookWithNoPlaybooksRegistered_ReturnsNull()
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

public class ArenaDisposeComponentTest : IClassFixture<ArenaDisposeComponentTest.Fixture>
{
    private readonly Fixture _fixture;

    public ArenaDisposeComponentTest(Fixture fixture)
    {
        _fixture = fixture;
    }

    public class Fixture : ArenaCollectionFixture
    {
        protected override Match Configure() => new MatchBuilder("lifecycle-dispose-match").Build();
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
    private static int _sharedMatchOpenCount = 0;

    internal class ConcreteFixture : ArenaCollectionFixture
    {
        protected override Match Configure()
        {
            _sharedMatchOpenCount++;
            return new MatchBuilder("shared-lifecycle-match").Build();
        }
    }

    [Fact]
    public void CollectionFixture_SharedMatch_OpensArenaOnce()
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
}
