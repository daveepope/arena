using System;
using System.Collections.Generic;
using System.Threading.Tasks;
using ArenaXunit.Xunit;

namespace ArenaXunit;

public abstract class ArenaCollectionFixture<TTopology> : IDisposable where TTopology : class, IArenaTopology, new()
{
    private static readonly object Lock = new object();
    private static readonly Dictionary<Type, SharedArena> SharedArenas = new Dictionary<Type, SharedArena>();

    public OpenArena Arena { get; private set; } = default!;

    public ArenaCollectionFixture()
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
    }

    public void Dispose()
    {
        var topologyType = typeof(TTopology);
        lock (Lock)
        {
            if (!SharedArenas.ContainsKey(topologyType))
                return;

            var shared = SharedArenas[topologyType];
            shared.RefCount--;

            if (shared.RefCount <= 0)
            {
                shared.Arena.Dispose();
                SharedArenas.Remove(topologyType);
            }
        }
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
