using System;
using System.Collections.Generic;
using System.Linq;
using System.Reflection;
using System.Threading;
using ArenaDotnet.Xunit.Playbook;

namespace ArenaDotnet.Xunit.Xunit;

public static class PlaybookScope
{
    private static readonly AsyncLocal<TestScope?> Scope = new();

    public static T GetActive<T>() where T : ActivePlaybook
    {
        var scope = Scope.Value;
        if (scope != null)
        {
            foreach (var playbook in scope.Actives)
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

        var playbookTypes = attributes.Select(a => a.PlaybookType).ToList();
        var (before, after) = PartitionPlaybooks(arena, playbookTypes);

        var actives = new List<ActivePlaybook>();
        var scope = new TestScope(arena, actives, after);
        Scope.Value = scope;
        try
        {
            foreach (var pb in before)
            {
                actives.Add(pb.Run(arena));
            }
        }
        catch (Exception activationError)
        {
            Scope.Value = null;

            List<Exception>? errors = null;
            errors = CollectDisposeErrors(actives, errors);
            errors = CollectManagedErrors(arena, after, errors);

            if (errors == null)
                throw;

            errors.Insert(0, activationError);
            throw new AggregateException(
                "playbook activation failed before the test body ran; one or more playbooks also failed cleanup",
                errors);
        }
    }

    public static void AfterTest(MethodInfo method, Type testClass)
    {
        var scope = Scope.Value;
        Scope.Value = null;
        if (scope == null)
            return;

        List<Exception>? errors = null;
        errors = CollectDisposeErrors(scope.Actives, errors);
        errors = CollectManagedErrors(scope.Arena, scope.ManagedPlaybooks, errors);

        ThrowIfAny(errors);
    }

    private static IPlaybook ResolveActivatable(OpenArena arena, Type playbookType)
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

        return pb;
    }

    private static (List<IPlaybook> Before, List<IPlaybook> After) PartitionPlaybooks(OpenArena arena, List<Type> playbookTypes)
    {
        var before = new List<IPlaybook>();
        var after = new List<IPlaybook>();
        foreach (var playbookType in playbookTypes)
        {
            var pb = ResolveActivatable(arena, playbookType);
            if (ActivatesBeforeTest(pb))
                before.Add(pb);
            else
                after.Add(pb);
        }
        return (before, after);
    }

    private static bool ActivatesBeforeTest(IPlaybook playbook)
    {
        if (playbook is ManagedPlaybook managed)
            return managed.ActivatesBeforeTest;
        return true;
    }

    private static List<Exception>? CollectManagedErrors(OpenArena arena, List<IPlaybook> managedPlaybooks, List<Exception>? errors)
    {
        foreach (var pb in managedPlaybooks)
        {
            try
            {
                using var active = pb.Run(arena);
            }
            catch (Exception ex)
            {
                (errors ??= new List<Exception>()).Add(ex);
            }
        }
        return errors;
    }

    private static List<Exception>? CollectDisposeErrors(List<ActivePlaybook> active, List<Exception>? errors)
    {
        for (var i = active.Count - 1; i >= 0; i--)
        {
            try
            {
                active[i].Dispose();
            }
            catch (Exception ex)
            {
                (errors ??= new List<Exception>()).Add(ex);
            }
        }
        active.Clear();
        return errors;
    }

    private static void ThrowIfAny(List<Exception>? errors)
    {
        if (errors == null)
            return;
        if (errors.Count == 1)
            System.Runtime.ExceptionServices.ExceptionDispatchInfo.Capture(errors[0]).Throw();
        throw new AggregateException("one or more playbooks failed verification on teardown", errors);
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

    private sealed class TestScope
    {
        public TestScope(OpenArena arena, List<ActivePlaybook> actives, List<IPlaybook> managedPlaybooks)
        {
            Arena = arena;
            Actives = actives;
            ManagedPlaybooks = managedPlaybooks;
        }

        public OpenArena Arena { get; }
        public List<ActivePlaybook> Actives { get; }
        public List<IPlaybook> ManagedPlaybooks { get; }
    }
}
