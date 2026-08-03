using System;
using System.Collections.Generic;
using System.Linq;
using System.Reflection;
using System.Threading;
using ArenaXunit.Playbook;

namespace ArenaXunit.Xunit;

public static class PlaybookScope
{
    private static readonly AsyncLocal<List<ActivePlaybook>?> ActivePlaybooks = new();

    public static T GetActive<T>() where T : ActivePlaybook
    {
        var active = ActivePlaybooks.Value;
        if (active != null)
        {
            foreach (var playbook in active)
            {
                if (playbook is T typed)
                    return typed;
            }
        }
        throw new InvalidOperationException(
            $"no active playbook of type {typeof(T).Name} for the current test; " +
            $"is the test decorated with [Playbook(typeof({typeof(T).Name}))]?");
    }

    public static void BeforeTest(MethodInfo method, Type testClass)
    {
        var attributes = GetPlaybookAttributes(method, testClass);
        if (!attributes.Any())
            return;

        var arena = ResolveOpenArenaFromStaticField(testClass);
        if (arena == null)
            return;

        var active = new List<ActivePlaybook>();
        ActivePlaybooks.Value = active;
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

                active.Add(pb.Run(arena));
            }
        }
        catch
        {
            DisposeAll(active);
            ActivePlaybooks.Value = null;
            throw;
        }
    }

    public static void AfterTest(MethodInfo method, Type testClass)
    {
        var active = ActivePlaybooks.Value;
        if (active == null)
            return;
        DisposeAll(active);
        ActivePlaybooks.Value = null;
    }

    private static void DisposeAll(List<ActivePlaybook> active)
    {
        for (var i = active.Count - 1; i >= 0; i--)
        {
            try
            {
                active[i].Dispose();
            }
            catch
            {
            }
        }
        active.Clear();
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
