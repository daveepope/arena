using System;
using System.Collections.Generic;
using System.Linq;
using System.Reflection;
using System.Threading.Tasks;
using ArenaXunit.Ffi;
using ArenaXunit.Playbook;
using ArenaXunit.Topology;
using Microsoft.Extensions.Logging;

namespace ArenaXunit;

public abstract class ArenaCollectionFixture<TTopology> : IDisposable where TTopology : class, new()
{
    private static readonly object Lock = new object();
    private static readonly Dictionary<Type, SharedArena> SharedArenas = new();

    public OpenArena Arena { get; private set; } = default!;

    protected ArenaCollectionFixture()
    {
        var topologyType = typeof(TTopology);
        lock (Lock)
        {
            var root = topologyRoot(topologyType);
            if (!SharedArenas.ContainsKey(root))
            {
                var closed = buildClosedArena(root);
                var openArena = closed.OpenAsync().GetAwaiter().GetResult();
                invokeAfterOpen(root, openArena);
                SharedArenas[root] = new SharedArena(openArena, 1);
            }
            else
            {
                SharedArenas[root].RefCount++;
            }
            Arena = SharedArenas[root].Arena;
        }
    }

    public void Dispose()
    {
        var topologyType = typeof(TTopology);
        lock (Lock)
        {
            var root = topologyRoot(topologyType);
            if (!SharedArenas.ContainsKey(root))
                return;

            var shared = SharedArenas[root];
            shared.RefCount--;

            if (shared.RefCount <= 0)
            {
                invokeBeforeClose(root);
                shared.Arena.Dispose();
                SharedArenas.Remove(root);
            }
        }
    }

    private static Type topologyRoot(Type type)
    {
        Type root = null;
        for (var current = type; current != null && current != typeof(object); current = current.BaseType!)
        {
            if (declaresArenaFields(current))
                root = current;
        }
        if (root == null)
            return type;
        return root;
    }

    private static bool declaresArenaFields(Type type)
    {
        foreach (var field in type.GetFields(BindingFlags.DeclaredOnly | BindingFlags.Static | BindingFlags.Public | BindingFlags.NonPublic))
        {
            if (field.IsDefined(typeof(ArenaDependencyAttribute), inherit: false)
                || field.IsDefined(typeof(ArenaComponentAttribute), inherit: false)
                || field.IsDefined(typeof(PlaybookAttribute), inherit: false))
            {
                return true;
            }
        }
        return false;
    }

    private static MatchBuilder buildMatchBuilder(Type root)
    {
        var matchBuilder = new MatchBuilder(root.Name);

        foreach (var field in root.GetFields(BindingFlags.DeclaredOnly | BindingFlags.Static | BindingFlags.Public | BindingFlags.NonPublic))
        {
            if (field.IsDefined(typeof(ArenaDependencyAttribute), inherit: false))
            {
                var dependency = readStatic<IArenaMatchPiece>(field, root);
                matchBuilder.AddDependency(dependency);
            }
            else if (field.IsDefined(typeof(ArenaComponentAttribute), inherit: false))
            {
                var component = readStatic<IArenaMatchPiece>(field, root);
                matchBuilder.AddComponent(component);
            }
            else if (field.IsDefined(typeof(PlaybookAttribute), inherit: false))
            {
                var attrs = field.GetCustomAttributes(typeof(PlaybookAttribute), inherit: false).Cast<PlaybookAttribute>().ToArray();
                foreach (var attr in attrs)
                {
                    var playbook = readStatic<IPlaybook>(field, root);
                    matchBuilder.RegisterPlaybook(playbook, attr.ExecOnDependencyStart);
                }
            }
        }

        return matchBuilder;
    }

    private static ClosedArena buildClosedArena(Type root)
    {
        var match = buildMatchBuilder(root).Build();

        var loggerField = findLoggerField(root);
        if (loggerField == null)
            return new ClosedArena(root.Name, match);

        var logger = readStatic<ILogger>(loggerField, root);
        var loggerAttr = loggerField.GetCustomAttribute<ArenaLoggerAttribute>(inherit: false);
        var logLevel = loggerAttr != null ? loggerAttr.Level : ArenaLogLevel.Info;

        return new ClosedArena(root.Name, match, logLevel, logger);
    }

    private static FieldInfo? findLoggerField(Type root)
    {
        FieldInfo? found = null;
        foreach (var field in root.GetFields(BindingFlags.DeclaredOnly | BindingFlags.Static | BindingFlags.Public | BindingFlags.NonPublic))
        {
            if (field.IsDefined(typeof(ArenaLoggerAttribute), inherit: false))
            {
                if (found != null)
                    throw new InvalidOperationException(
                        $"ArenaCollectionFixture: multiple @ArenaLogger fields on {root.Name}");
                found = field;
            }
        }
        return found;
    }

    private static void invokeAfterOpen(Type root, OpenArena openArena)
    {
        var method = findLifecycleMethod(root, typeof(ArenaAfterOpenAttribute));
        if (method == null)
            return;

        if (!method.IsStatic)
            throw new InvalidOperationException(
                $"@ArenaAfterOpen method must be static: {root.Name}.{method.Name}");

        method.Invoke(null, method.GetParameters().Length > 0 ? new object[] { openArena } : null);
    }

    private static void invokeBeforeClose(Type root)
    {
        var method = findLifecycleMethod(root, typeof(ArenaBeforeCloseAttribute));
        if (method == null)
            return;

        if (!method.IsStatic)
            throw new InvalidOperationException(
                $"@ArenaBeforeClose method must be static: {root.Name}.{method.Name}");

        method.Invoke(null, null);
    }

    private static MethodInfo? findLifecycleMethod(Type root, Type attributeType)
    {
        MethodInfo? found = null;
        foreach (var method in root.GetMethods(BindingFlags.DeclaredOnly | BindingFlags.Static | BindingFlags.Public | BindingFlags.NonPublic))
        {
            if (method.IsDefined(attributeType, inherit: false))
            {
                if (found != null)
                    throw new InvalidOperationException(
                        $"ArenaCollectionFixture<{root.Name}>: multiple {attributeType.Name} methods on {root.Name}");
                found = method;
            }
        }
        return found;
    }

    private static T readStatic<T>(FieldInfo field, Type root)
    {
        if (!field.IsStatic)
            throw new InvalidOperationException(
                $"@Arena fields must be static: {root.Name}.{field.Name}");

        var value = field.GetValue(null);
        if (value is not T typed)
            throw new InvalidOperationException(
                $"@Arena field {root.Name}.{field.Name} must be of type {typeof(T).Name}");
        return typed;
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
