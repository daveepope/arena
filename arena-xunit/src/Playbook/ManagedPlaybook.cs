using System;
using ArenaDotnet.Xunit.Ffi;

namespace ArenaDotnet.Xunit.Playbook;

public abstract class ManagedPlaybook : IPlaybook
{
    public string Identifier { get; }
    public string DependencyIdentifier { get; }

    protected ManagedPlaybook(string identifier, string dependencyIdentifier)
    {
        if (string.IsNullOrEmpty(identifier))
            throw new ArgumentException("identifier must not be null or empty", nameof(identifier));
        if (string.IsNullOrEmpty(dependencyIdentifier))
            throw new ArgumentException("dependencyIdentifier must not be null or empty", nameof(dependencyIdentifier));
        Identifier = identifier;
        DependencyIdentifier = dependencyIdentifier;
    }

    public ActivePlaybook Run(OpenArena arena)
    {
        return WrapHandle(ArenaBindings.MatchPlaybookRun(arena.Handle, Identifier));
    }

    internal abstract string Kind { get; }

    internal virtual object BuildRegistrationConfig(bool execOnDependencyStart)
    {
        return new
        {
            identifier = Identifier,
            kind = Kind,
            dependency_identifier = DependencyIdentifier,
            exec_on_dependency_start = execOnDependencyStart,
        };
    }

    internal abstract ActivePlaybook WrapHandle(IntPtr handle);
}
