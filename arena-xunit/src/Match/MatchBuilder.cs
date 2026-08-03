using System;
using System.Collections.Generic;

namespace ArenaXunit;

public sealed class MatchBuilder
{
    private readonly string _name;
    private readonly List<IArenaMatchPiece> _dependencies = new();
    private readonly List<IArenaMatchPiece> _components = new();
    private string? _network;
    private readonly List<RegisteredPlaybook> _playbooks = new();

    public MatchBuilder(string name)
    {
        _name = name ?? throw new ArgumentNullException(nameof(name));
    }

    public MatchBuilder WithNetwork(string network)
    {
        _network = network;
        return this;
    }

    public MatchBuilder AddDependency(IArenaMatchPiece dependency)
    {
        _dependencies.Add(dependency);
        return this;
    }

    public MatchBuilder AddComponent(IArenaMatchPiece component)
    {
        _components.Add(component);
        return this;
    }

    public MatchBuilder RegisterPlaybook(Playbook.IPlaybook playbook, bool execOnDependencyStart)
    {
        _playbooks.Add(new RegisteredPlaybook(playbook, execOnDependencyStart));
        return this;
    }

    public Match Build()
    {
        return new Match(_name, _dependencies.AsReadOnly(), _components.AsReadOnly(), _network, _playbooks.AsReadOnly());
    }
}

public sealed class RegisteredPlaybook
{
    public RegisteredPlaybook(Playbook.IPlaybook playbook, bool execOnDependencyStart)
    {
        Playbook = playbook;
        ExecOnDependencyStart = execOnDependencyStart;
    }

    public Playbook.IPlaybook Playbook { get; }
    public bool ExecOnDependencyStart { get; }

    public object ToConfig()
    {
        if (Playbook is Playbook.ManagedPlaybook managed)
            return managed.BuildRegistrationConfig(ExecOnDependencyStart);

        return new
        {
            identifier = Playbook.Identifier,
            kind = "unknown",
            exec_on_dependency_start = ExecOnDependencyStart,
        };
    }
}
