using ArenaXunit.Topology;
using ArenaXunit.Support;
using Newtonsoft.Json;

namespace ArenaXunit.Dep;

public sealed class TemporalDependency : IArenaMatchPiece
{
    public string Type => "temporal";
    public string Identifier { get; }
    public int Port { get; }

    internal TemporalDependency(string identifier, int port)
    {
        Identifier = identifier;
        Port = port;
    }

    public string ForFfi()
    {
        return ArenaJson.Serialize(new TemporalConfig
        {
            Type = Type,
            Identifier = Identifier,
            Port = Port,
        });
    }

    [JsonObject(ItemNullValueHandling = NullValueHandling.Ignore)]
    private sealed class TemporalConfig
    {
        [JsonProperty("type")] public string Type { get; set; } = default!;
        [JsonProperty("identifier")] public string Identifier { get; set; } = default!;
        [JsonProperty("port")] public int Port { get; set; }
    }
}

public sealed class TemporalDependencyBuilder
{
    private readonly string _name;
    private int _port = 7233;

    public TemporalDependencyBuilder(string name)
    {
        _name = name;
    }

    public TemporalDependencyBuilder WithPort(int port)
    {
        _port = port;
        return this;
    }

    public TemporalDependency Build()
    {
        var identifier = ArenaIdentifiers.Build("arena-temporal", _name);
        return new TemporalDependency(identifier, _port);
    }
}
