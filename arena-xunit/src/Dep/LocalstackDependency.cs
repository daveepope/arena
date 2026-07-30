using ArenaXunit.Topology;
using ArenaXunit.Support;
using Newtonsoft.Json;

namespace ArenaXunit.Dep;

public sealed class LocalstackDependency : IArenaMatchPiece
{
    public string Type => "localstack";
    public string Identifier { get; }
    public int Port { get; }

    internal LocalstackDependency(string identifier, int port)
    {
        Identifier = identifier;
        Port = port;
    }

    public string ForFfi()
    {
        return ArenaJson.Serialize(new LocalstackConfig
        {
            Type = Type,
            Identifier = Identifier,
            Port = Port,
        });
    }

    [JsonObject(ItemNullValueHandling = NullValueHandling.Ignore)]
    private sealed class LocalstackConfig
    {
        [JsonProperty("type")] public string Type { get; set; } = default!;
        [JsonProperty("identifier")] public string Identifier { get; set; } = default!;
        [JsonProperty("port")] public int Port { get; set; }
    }
}

public sealed class LocalstackDependencyBuilder
{
    private readonly string _name;
    private int _port = 4566;

    public LocalstackDependencyBuilder(string name)
    {
        _name = name;
    }

    public LocalstackDependencyBuilder WithPort(int port)
    {
        _port = port;
        return this;
    }

    public LocalstackDependency Build()
    {
        var identifier = ArenaIdentifiers.Build("arena-localstack", _name);
        return new LocalstackDependency(identifier, _port);
    }
}
