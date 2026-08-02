using ArenaXunit.Support;

namespace ArenaXunit.Dep;

public sealed class PostgresDependency : IArenaMatchPiece
{
    public string Type => "postgres";
    public string Identifier { get; }
    public int Port { get; }

    internal PostgresDependency(string identifier, int port)
    {
        Identifier = identifier;
        Port = port;
    }

    public string ForFfi()
    {
        return ArenaJson.Serialize(this);
    }
}

public sealed class PostgresDependencyBuilder
{
    private readonly string _name;
    private int _port = 5432;

    public PostgresDependencyBuilder(string name)
    {
        _name = name;
    }

    public PostgresDependencyBuilder WithPort(int port)
    {
        _port = port;
        return this;
    }

    public PostgresDependency Build()
    {
        var identifier = ArenaIdentifiers.Build("arena-postgres", _name);
        return new PostgresDependency(identifier, _port);
    }
}
