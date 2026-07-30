using ArenaXunit.Topology;
using ArenaXunit.Support;
using Newtonsoft.Json;

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
        return ArenaJson.Serialize(new PostgresConfig
        {
            Type = Type,
            Identifier = Identifier,
            Port = Port,
        });
    }

    [JsonObject(ItemNullValueHandling = NullValueHandling.Ignore)]
    private sealed class PostgresConfig
    {
        [JsonProperty("type")] public string Type { get; set; } = default!;
        [JsonProperty("identifier")] public string Identifier { get; set; } = default!;
        [JsonProperty("port")] public int Port { get; set; }
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
