using System;
using System.Collections.Generic;
using System.Linq;
using System.Reflection;
using ArenaXunit.Playbook;

namespace ArenaXunit.Xunit;

internal static class PlaybookScope
{
    private static readonly object Lock = new object();
    private static readonly Dictionary<string, List<ActivePlaybook>> _activePlaybooks = new();

    public static void BeforeTest(MethodInfo method, Type testClass)
    {
        var attributes = GetPlaybookAttributes(method, testClass);
        if (!attributes.Any())
            return;

        var arena = ResolveOpenArenaFromStaticField(testClass);
        if (arena == null)
            return;

        var scopeKey = BuildScopeKey(testClass, method);
        try
        {
            foreach (var playbookType in attributes.Select(a => a.PlaybookType))
            {
                var pb = arena.GetPlaybook(playbookType);
                if (pb == null)
                {
                    throw new InvalidOperationException(
                        $"[Playbook]: no playbook of type {playbookType.Name} is registered on any match");
                }

                var execOnStart = arena.PlaybookExecOnDependencyStart(playbookType);
                if (execOnStart)
                {
                    throw new InvalidOperationException(
                        $"[Playbook]: playbook {playbookType.Name} was registered with execOnDependencyStart=true and cannot be scoped per-test");
                }

                var active = pb.Run(arena);
                lock (Lock)
                {
                    if (!_activePlaybooks.ContainsKey(scopeKey))
                    {
                        _activePlaybooks[scopeKey] = new List<ActivePlaybook>();
                    }
                    _activePlaybooks[scopeKey].Add(active);
                }
            }
        }
        catch
        {
            DisposeAll(scopeKey);
            throw;
        }
    }

    public static void AfterTest(MethodInfo method, Type testClass)
    {
        var scopeKey = BuildScopeKey(testClass, method);
        DisposeAll(scopeKey);
    }

    private static void DisposeAll(string scopeKey)
    {
        lock (Lock)
        {
            if (_activePlaybooks.TryGetValue(scopeKey, out var list))
            {
                for (var i = list.Count - 1; i >= 0; i--)
                {
                    try
                    {
                        list[i].Dispose();
                    }
                    catch
                    {
                    }
                }
                _activePlaybooks.Remove(scopeKey);
            }
        }
    }

    private static IEnumerable<PlaybookAttribute> GetPlaybookAttributes(MethodInfo method, Type testClass)
    {
        var methodAttrs = method.GetCustomAttributes(typeof(PlaybookAttribute), true)
            .Cast<PlaybookAttribute>();
        if (methodAttrs.Any())
            return methodAttrs;

        var classAttrs = testClass.GetCustomAttributes(typeof(PlaybookAttribute), true)
            .Cast<PlaybookAttribute>();
        return classAttrs;
    }

    private static string BuildScopeKey(Type testClass, MethodInfo method)
    {
        return $"{testClass.FullName}.{method.Name}";
    }

    private static OpenArena? ResolveOpenArenaFromStaticField(Type testClass)
    {
        foreach (var prop in testClass.GetProperties(BindingFlags.Static | BindingFlags.Public | BindingFlags.NonPublic))
        {
            if (prop.PropertyType == typeof(OpenArena))
            {
                return prop.GetValue(null) as OpenArena;
            }
        }
        return null;
    }
}
