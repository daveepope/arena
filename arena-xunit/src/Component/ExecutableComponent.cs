using System.Collections.Generic;
using ArenaXunit.Support;

namespace ArenaXunit.Component;

public sealed class ExecutableComponent : IArenaMatchPiece
{
    public string Type => "exec";
    public string Identifier { get; }
    public string ExecutablePath { get; }
    public List<string>? Args { get; }
    public Dictionary<string, string>? Env { get; }

    internal ExecutableComponent(string identifier, string executablePath, List<string>? args,
        Dictionary<string, string>? env)
    {
        Identifier = identifier;
        ExecutablePath = executablePath;
        Args = args;
        Env = env;
    }

    public string ForFfi()
    {
        return ArenaJson.Serialize(this);
    }
}

public sealed class ExecutableComponentBuilder
{
    private readonly string _name;
    private string? _executablePath;
    private readonly List<string> _args = new();
    private readonly Dictionary<string, string> _env = new();

    public ExecutableComponentBuilder(string name)
    {
        _name = name;
    }

    public ExecutableComponentBuilder WithExecutablePath(string path)
    {
        _executablePath = path;
        return this;
    }

    public ExecutableComponentBuilder WithArgs(params string[] args)
    {
        foreach (var arg in args)
            _args.Add(arg);
        return this;
    }

    public ExecutableComponentBuilder WithEnv(string key, string value)
    {
        _env[key] = value;
        return this;
    }

    public ExecutableComponent Build()
    {
        if (string.IsNullOrEmpty(_executablePath))
            throw new System.InvalidOperationException("executable path must be set");
        var identifier = ArenaIdentifiers.Build("arena-exec", _name);
        return new ExecutableComponent(identifier, _executablePath,
            _args.Count > 0 ? new List<string>(_args) : null,
            _env.Count > 0 ? new Dictionary<string, string>(_env) : null);
    }
}
