using System.Collections.Generic;
using Newtonsoft.Json;

namespace ArenaXunit.Playbook;

public abstract class ManagedHttpPlaybook : IPlaybook
{
    public string Identifier { get; }
    public string DependencyIdentifier { get; }
    public List<object> Mappings { get; }

    protected ManagedHttpPlaybook(string identifier, string dependencyIdentifier, HttpPlaybookBuilder builder)
    {
        Identifier = identifier;
        DependencyIdentifier = dependencyIdentifier;
        Mappings = builder.BuildMappings();
    }

    public ActivePlaybook Run(OpenArena arena)
    {
        return new ActiveHttpPlaybook(IntPtr.Zero);
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
        return new ActiveMssqlPlaybook(IntPtr.Zero);
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
        return new ActivePlaybook(IntPtr.Zero);
    }
}
