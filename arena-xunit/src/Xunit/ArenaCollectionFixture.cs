using System;
using System.Collections.Concurrent;
using System.Collections.Generic;
using System.Threading;

namespace ArenaXunit;

public abstract class ArenaCollectionFixture : IDisposable
{
    private static readonly ConcurrentDictionary<Type, Lazy<SharedArena>> SharedArenas = new();

    private readonly Lazy<SharedArena> _sharedEntry;
    private readonly SharedArena _shared;

    protected ArenaCollectionFixture()
    {
        var key = GetType();
        while (true)
        {
            var lazy = SharedArenas.GetOrAdd(
                key,
                _ => new Lazy<SharedArena>(OpenSharedArena, LazyThreadSafetyMode.ExecutionAndPublication));
            var shared = lazy.Value;
            if (shared.TryAddRef())
            {
                _sharedEntry = lazy;
                _shared = shared;
                return;
            }

            RemoveStaleEntry(key, lazy);
        }
    }

    public OpenArena Arena => _shared.Arena;

    protected abstract Match Configure();

    public void Dispose()
    {
        if (!_shared.Release())
            return;

        RemoveStaleEntry(GetType(), _sharedEntry);
    }

    private static void RemoveStaleEntry(Type key, Lazy<SharedArena> lazy)
    {
        ((ICollection<KeyValuePair<Type, Lazy<SharedArena>>>)SharedArenas)
            .Remove(new KeyValuePair<Type, Lazy<SharedArena>>(key, lazy));
    }

    private SharedArena OpenSharedArena()
    {
        var match = Configure();
        var closed = new ClosedArena(GetType().Name, match);
        var arena = closed.OpenAsync().GetAwaiter().GetResult();
        return new SharedArena(arena);
    }

    private sealed class SharedArena
    {
        private readonly object _lock = new object();
        private int _refCount;
        private bool _disposed;

        public SharedArena(OpenArena arena)
        {
            Arena = arena;
        }

        public OpenArena Arena { get; }

        public bool TryAddRef()
        {
            lock (_lock)
            {
                if (_disposed)
                    return false;
                _refCount++;
                return true;
            }
        }

        public bool Release()
        {
            lock (_lock)
            {
                _refCount--;
                if (_refCount > 0)
                    return false;
                _disposed = true;
                Arena.Dispose();
                return true;
            }
        }
    }
}
