using System;
using System.Collections.Generic;
using System.Threading.Tasks;

namespace ArenaXunit;

public abstract class ArenaCollectionFixture<TTopology> : IAsyncLifetime where TTopology : class, IArenaTopology, new()
{
    private static readonly object Lock = new object();
    private static readonly Dictionary<Type, SharedArena> SharedArenas = new Dictionary<Type>();

    public OpenArena Arena { get; private set; } = default!;

    public Task InitializeAsync()
    {
        var topologyType = typeof(TTopology);
        lock (Lock)
        {
            if (!SharedArenas.ContainsKey(topologyType))
            {
                var topology = new TTopology();
                var match = topology.Configure();
                var closed = new ClosedArena(topologyType.Name, match);
                var arena = closed.OpenAsync().GetAwaiter().GetResult();
                SharedArenas[topologyType] = new SharedArena(arena, 1);
            }
            else
            {
                SharedArenas[topologyType].RefCount++;
            }
            Arena = SharedArenas[topologyType].Arena;
        }
        return Task.CompletedTask;
    }

    public Task DisposeAsync()
    {
        var topologyType = typeof(TTopology);
        lock (Lock)
        {
            if (!SharedArenas.ContainsKey(topologyType))
                return Task.CompletedTask;

            var shared = SharedArenas[topologyType];
            shared.RefCount--;

            if (shared.RefCount <= 0)
            {
                shared.Arena.Dispose();
                SharedArenas.Remove(topologyType);
            }
        }
        return Task.CompletedTask;
    }

    private sealed class SharedArena
    {
        public SharedArena(OpenArena arena, int refCount)
        {
            Arena = arena;
            RefCount = refCount;
        }

        public OpenArena Arena { get; }
        public int RefCount { get; set; }
    }
}
