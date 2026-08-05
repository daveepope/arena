using System;
using System.Collections.Concurrent;
using System.Collections.Generic;
using System.Reflection;
using System.Threading;
using ArenaDotnet.Xunit.Ffi;
using Microsoft.Extensions.Logging;

namespace ArenaDotnet.Xunit;

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

    protected virtual Match Configure()
    {
        return BuildMatchFromAttributes();
    }

    private Match BuildMatchFromAttributes()
    {
        var type = GetType();
        var builder = new MatchBuilder(type.Name);
        var foundAny = false;

        for (var current = type; current != null && current != typeof(object); current = current.BaseType)
        {
            foreach (var field in current.GetFields(BindingFlags.Static | BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.DeclaredOnly))
            {
                if (field.GetCustomAttribute<ArenaDependencyAttribute>() != null)
                {
                    builder.AddDependency((IArenaMatchPiece)RequireFieldValue(field));
                    foundAny = true;
                }
                else if (field.GetCustomAttribute<ArenaComponentAttribute>() != null)
                {
                    builder.AddComponent((IArenaMatchPiece)RequireFieldValue(field));
                    foundAny = true;
                }
                else if (field.GetCustomAttribute<ArenaPlaybookAttribute>() is { } playbookAttribute)
                {
                    builder.RegisterPlaybook((Playbook.IPlaybook)RequireFieldValue(field), playbookAttribute.ExecOnDependencyStart);
                    foundAny = true;
                }
            }
        }

        if (!foundAny)
        {
            throw new InvalidOperationException(
                $"{type.Name} does not override Configure() and declares no [ArenaDependency]/[ArenaComponent]/[ArenaPlaybook] static fields");
        }

        return builder.Build();
    }

    private static object RequireFieldValue(FieldInfo field)
    {
        return field.GetValue(null)
            ?? throw new InvalidOperationException($"[Arena*] field '{field.DeclaringType!.Name}.{field.Name}' must not be null");
    }

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
        var (logger, level) = ResolveLogger();
        var (dependencyIds, componentIds) = CollectLogIdentifiers();
        var closed = new ClosedArena(GetType().Name, match, level, logger, dependencyIds, componentIds);
        var arena = closed.OpenAsync().GetAwaiter().GetResult();
        return new SharedArena(arena);
    }

    internal (List<string> DependencyIds, List<string> ComponentIds) CollectLogIdentifiers()
    {
        var dependencyIds = new List<string>();
        var componentIds = new List<string>();

        for (var current = GetType(); current != null && current != typeof(object); current = current.BaseType)
        {
            foreach (var field in current.GetFields(BindingFlags.Static | BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.DeclaredOnly))
            {
                var dependency = field.GetCustomAttribute<ArenaDependencyAttribute>();
                if (dependency != null && dependency.Logs)
                {
                    dependencyIds.Add(IdentifierOf(field));
                }

                var component = field.GetCustomAttribute<ArenaComponentAttribute>();
                if (component != null && component.Logs)
                {
                    componentIds.Add(IdentifierOf(field));
                }
            }
        }

        return (dependencyIds, componentIds);
    }

    private static string IdentifierOf(FieldInfo field)
    {
        return ((IArenaMatchPiece)RequireFieldValue(field)).Identifier;
    }

    private (ILogger? Logger, ArenaLogLevel Level) ResolveLogger()
    {
        FieldInfo? found = null;

        for (var current = GetType(); current != null && current != typeof(object); current = current.BaseType)
        {
            foreach (var field in current.GetFields(BindingFlags.Static | BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.DeclaredOnly))
            {
                if (field.GetCustomAttribute<ArenaLoggerAttribute>() == null)
                    continue;
                if (found != null)
                {
                    throw new InvalidOperationException(
                        $"multiple [ArenaLogger] fields found on {GetType().Name}");
                }
                found = field;
            }
        }

        if (found == null)
            return (null, ArenaLogLevel.Info);

        var logger = (ILogger)RequireFieldValue(found);
        var level = found.GetCustomAttribute<ArenaLoggerAttribute>()!.Level;
        return (logger, level);
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
