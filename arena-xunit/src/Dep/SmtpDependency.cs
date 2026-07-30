using ArenaXunit.Topology;
using ArenaXunit.Support;
using Newtonsoft.Json;

namespace ArenaXunit.Dep;

public sealed class SmtpDependency : IArenaMatchPiece
{
    public string Type => "smtp";
    public string Identifier { get; }
    public int Port { get; }

    internal SmtpDependency(string identifier, int port)
    {
        Identifier = identifier;
        Port = port;
    }

    public string ForFfi()
    {
        return ArenaJson.Serialize(new SmtpConfig
        {
            Type = Type,
            Identifier = Identifier,
            Port = Port,
        });
    }

    [JsonObject(ItemNullValueHandling = NullValueHandling.Ignore)]
    private sealed class SmtpConfig
    {
        [JsonProperty("type")] public string Type { get; set; } = default!;
        [JsonProperty("identifier")] public string Identifier { get; set; } = default!;
        [JsonProperty("port")] public int Port { get; set; }
    }
}

public sealed class SmtpDependencyBuilder
{
    private readonly string _name;
    private int _port = 1025;

    public SmtpDependencyBuilder(string name)
    {
        _name = name;
    }

    public SmtpDependencyBuilder WithPort(int port)
    {
        _port = port;
        return this;
    }

    public SmtpDependency Build()
    {
        var identifier = ArenaIdentifiers.Build("arena-smtp", _name);
        return new SmtpDependency(identifier, _port);
    }
}
