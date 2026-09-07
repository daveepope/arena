using System;
using System.Collections.Generic;
using ArenaDotnet.Xunit.Support;
using Newtonsoft.Json;

namespace ArenaDotnet.Xunit.Lifecycle;

public sealed class ArenaState
{
    public const string ArenaFaulted = "arena_faulted";
    public const string ArenaClosed = "arena_closed";

    public string Id { get; set; } = "";
    public string State { get; set; } = "";
    public string At { get; set; } = "";
    public List<DependencyState> Dependencies { get; set; } = new();
    public List<ComponentState> Components { get; set; } = new();
    public List<Fault> Faults { get; set; } = new();

    public static ArenaState Parse(string document)
    {
        try
        {
            return ArenaJson.Deserialize<ArenaState>(document);
        }
        catch (InvalidOperationException e)
        {
            throw new ArgumentException("arena state document must be a json object", e);
        }
        catch (JsonException e)
        {
            throw new ArgumentException("arena state document failed to parse", e);
        }
    }

    public bool IsFaulted() => State == ArenaFaulted;

    public DependencyState? Dependency(string identifier)
    {
        foreach (var dep in Dependencies)
        {
            var found = dep.Find(identifier);
            if (found != null)
                return found;
        }
        return null;
    }

    public ComponentState? Component(string identifier)
    {
        foreach (var comp in Components)
        {
            var found = comp.Find(identifier);
            if (found != null)
                return found;
        }
        return null;
    }
}
