using System;
using System.Collections.Generic;

namespace ArenaDotnet.Xunit;

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
        if (playbook == null)
            throw new ArgumentNullException(nameof(playbook));
        if (playbook is not Playbook.ManagedPlaybook and not Playbook.UnmanagedPlaybook)
        {
            throw new ArgumentException(
                $"RegisterPlaybook requires a ManagedPlaybook or UnmanagedPlaybook instance " +
                $"(got {playbook.GetType().Name})",
                nameof(playbook));
        }
        if (execOnDependencyStart && playbook is not Playbook.ManagedPlaybook)
        {
            throw new ArgumentException(
                $"RegisterPlaybook(..., execOnDependencyStart: true) requires a playbook that " +
                $"serializes its manifest (a ManagedPlaybook subclass); {playbook.GetType().Name} does not",
                nameof(execOnDependencyStart));
        }

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

    public object? ToConfig()
    {
        if (Playbook is Playbook.ManagedPlaybook managed)
            return managed.BuildRegistrationConfig(ExecOnDependencyStart);

        return null;
    }
}
