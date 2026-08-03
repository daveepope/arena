using System;
using System.Collections.Generic;

namespace ArenaXunit.Playbook;

public abstract class ManagedHttpPlaybook : ManagedPlaybook
{
    public List<object> Mappings { get; }

    protected ManagedHttpPlaybook(string identifier, string dependencyIdentifier, List<object> mappings)
        : base(identifier, dependencyIdentifier)
    {
        Mappings = mappings;
    }

    internal override string Kind => "http";

    internal override object BuildRegistrationConfig(bool execOnDependencyStart)
    {
        return new
        {
            identifier = Identifier,
            kind = Kind,
            dependency_identifier = DependencyIdentifier,
            mappings = Mappings,
            exec_on_dependency_start = execOnDependencyStart,
        };
    }

    internal override ActivePlaybook WrapHandle(IntPtr handle) => new ActiveHttpPlaybook(handle);
}
