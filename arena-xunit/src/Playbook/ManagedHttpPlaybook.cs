using System;
using System.Collections.Generic;
using ArenaXunit.Ffi;

namespace ArenaXunit.Playbook;

public abstract class ManagedHttpPlaybook : IPlaybook
{
    public string Identifier { get; }
    public string DependencyIdentifier { get; }
    public List<object> Mappings { get; }

    protected ManagedHttpPlaybook(string identifier, string dependencyIdentifier, List<object> mappings)
    {
        Identifier = identifier;
        DependencyIdentifier = dependencyIdentifier;
        Mappings = mappings;
    }

    public ActivePlaybook Run(OpenArena arena)
    {
        var handle = Ffi.ArenaNativeLib.arena_match_playbook_run(arena.Handle, Identifier, out var errOut);
        if (handle == IntPtr.Zero)
            throw Ffi.ArenaBindings.TakeErr(errOut, "arena_match_playbook_run failed");
        return new ActiveHttpPlaybook(handle);
    }
}

public abstract class ManagedMssqlPlaybook : IPlaybook
{
    public string Identifier { get; }
    public string DependencyIdentifier { get; }

    protected ManagedMssqlPlaybook(string identifier, string dependencyIdentifier)
    {
        Identifier = identifier;
        DependencyIdentifier = dependencyIdentifier;
    }

    public ActivePlaybook Run(OpenArena arena)
    {
        var handle = Ffi.ArenaNativeLib.arena_match_playbook_run(arena.Handle, Identifier, out var errOut);
        if (handle == IntPtr.Zero)
            throw Ffi.ArenaBindings.TakeErr(errOut, "arena_match_playbook_run failed");
        return new ActiveMssqlPlaybook(handle);
    }
}

public abstract class ManagedLocalstackPlaybook : IPlaybook
{
    public string Identifier { get; }
    public string DependencyIdentifier { get; }

    protected ManagedLocalstackPlaybook(string identifier, string dependencyIdentifier)
    {
        Identifier = identifier;
        DependencyIdentifier = dependencyIdentifier;
    }

    public ActivePlaybook Run(OpenArena arena)
    {
        var handle = Ffi.ArenaNativeLib.arena_match_playbook_run(arena.Handle, Identifier, out var errOut);
        if (handle == IntPtr.Zero)
            throw Ffi.ArenaBindings.TakeErr(errOut, "arena_match_playbook_run failed");
        return new ActiveLocalstackPlaybook(handle);
    }
}
