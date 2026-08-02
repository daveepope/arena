using System.Collections.Generic;
using ArenaXunit.Support;

namespace ArenaXunit.Component;

public sealed class ContainerizedComponent : IArenaMatchPiece
{
    public string Type => "container";
    public string Identifier { get; }
    public string Containerfile { get; }
    public Dictionary<string, string>? Env { get; }
    public List<string>? Args { get; }

    internal ContainerizedComponent(string identifier, string containerfile,
        Dictionary<string, string>? env, List<string>? args)
    {
        Identifier = identifier;
        Containerfile = containerfile;
        Env = env;
        Args = args;
    }

    public string ForFfi()
    {
        return ArenaJson.Serialize(this);
    }
}

public sealed class ContainerizedComponentBuilder
{
    private readonly string _name;
    private string? _containerfile;
    private readonly Dictionary<string, string> _env = new();
    private readonly List<string> _args = new();

    public ContainerizedComponentBuilder(string name)
    {
        _name = name;
    }

    public ContainerizedComponentBuilder WithContainerfile(string path)
    {
        _containerfile = path;
        return this;
    }

    public ContainerizedComponentBuilder WithEnv(string key, string value)
    {
        _env[key] = value;
        return this;
    }

    public ContainerizedComponentBuilder WithArgs(params string[] args)
    {
        foreach (var arg in args)
            _args.Add(arg);
        return this;
    }

    public ContainerizedComponent Build()
    {
        if (string.IsNullOrEmpty(_containerfile))
            throw new System.InvalidOperationException("containerfile must be set");
        var identifier = ArenaIdentifiers.Build("arena-container", _name);
        return new ContainerizedComponent(identifier, _containerfile,
            _env.Count > 0 ? new Dictionary<string, string>(_env) : null,
            _args.Count > 0 ? new List<string>(_args) : null);
    }
}
