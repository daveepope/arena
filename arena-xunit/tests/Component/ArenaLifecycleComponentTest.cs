using System;
using System.Net;
using System.Net.Sockets;
using System.Threading.Tasks;
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

public class ArenaLifecycleComponentTest
{
    private static int _failures = 0;

    public static async Task RunAll()
    {
        await RunTest("openArena_withEmptyMatch_opensAndClosesSuccessfully", async () => await openArena_withEmptyMatch_opensAndClosesSuccessfully());
        await RunTest("openArena_withOauthDependency_opensAndClosesSuccessfully", async () => await openArena_withOauthDependency_opensAndClosesSuccessfully());
        await RunTest("openArena_getPlaybook_withNoPlaybooksRegistered_returnsNull", async () => await openArena_getPlaybook_withNoPlaybooksRegistered_returnsNull());
        await RunTest("openArena_dispose_canBeCalledMultipleTimes", async () => await openArena_dispose_canBeCalledMultipleTimes());
        await RunTest("openArena_methodsAfterDispose_throwObjectDisposedException", async () => await openArena_methodsAfterDispose_throwObjectDisposedException());
        await RunTest("collectionFixture_opensArenaOnceForSharedTopology", async () => await collectionFixture_opensArenaOnceForSharedTopology());

        Console.WriteLine($"Tests completed: failures={_failures}");
        Environment.Exit(_failures > 0 ? 1 : 0);
    }

    private static async Task RunTest(string name, Func<Task> test)
    {
        try
        {
            await test();
            Console.WriteLine($"PASSED: {name}");
        }
        catch (Exception ex)
        {
            Console.WriteLine($"FAILED: {name}: {ex.Message}");
            _failures++;
        }
    }

    public static async Task openArena_withEmptyMatch_opensAndClosesSuccessfully()
    {
        var match = new MatchBuilder("lifecycle-empty-match")
            .Build();

        var closed = new ClosedArena("test-arena-empty", match);
        var arena = await closed.OpenAsync();
        Assert.NotNull(arena);

        arena.Dispose();
    }

    public static async Task openArena_withOauthDependency_opensAndClosesSuccessfully()
    {
        var port = TestRuntime.AllocatePort();

        var match = new MatchBuilder("lifecycle-oauth-match")
            .AddDependency(new OauthDependencyBuilder("test-oauth")
                .WithPort(port)
                .WithListenIp("0.0.0.0")
                .Build())
            .Build();

        var closed = new ClosedArena("test-arena-oauth", match);
        var arena = await closed.OpenAsync();
        Assert.NotNull(arena);

        arena.Dispose();
    }

    public static async Task openArena_getPlaybook_withNoPlaybooksRegistered_returnsNull()
    {
        var match = new MatchBuilder("lifecycle-no-playbooks")
            .Build();

        var closed = new ClosedArena("test-arena-no-pbs", match);
        var arena = await closed.OpenAsync();

        var playbook = arena.GetPlaybook(typeof(object));
        Assert.Null(playbook);

        arena.Dispose();
    }

    public static async Task openArena_dispose_canBeCalledMultipleTimes()
    {
        var match = new MatchBuilder("lifecycle-dispose-twice")
            .Build();

        var closed = new ClosedArena("test-arena-dispose-twice", match);
        var arena = await closed.OpenAsync();

        arena.Dispose();
        arena.Dispose();
    }

    public static async Task openArena_methodsAfterDispose_throwObjectDisposedException()
    {
        var match = new MatchBuilder("lifecycle-disposed-error")
            .Build();

        var closed = new ClosedArena("test-arena-disposed", match);
        var arena = await closed.OpenAsync();
        arena.Dispose();

        Assert.Throws<ObjectDisposedException>(() => arena.GetPlaybook(typeof(object)));
    }

    private static int _sharedTopologyOpenCount = 0;

    public class SharedTopologyConfig : IArenaTopology
    {
        public Match Configure()
        {
            _sharedTopologyOpenCount++;
            return new MatchBuilder("shared-lifecycle-match").Build();
        }
    }

    public static async Task collectionFixture_opensArenaOnceForSharedTopology()
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

    private class ConcreteFixture : ArenaCollectionFixture<SharedTopologyConfig>
    {
    }
}
