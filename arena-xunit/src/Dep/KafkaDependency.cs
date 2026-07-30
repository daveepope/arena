using ArenaXunit.Match;
using ArenaXunit.Support;
using Newtonsoft.Json;

namespace ArenaXunit.Dep;

public enum KafkaFlavor
{
    Zookeeper,
    KRaft
}

public sealed class KafkaDependency : IArenaMatchPiece
{
    public string Type => "kafka";
    public string Identifier { get; }
    public int Port { get; }
    public KafkaFlavor Flavor { get; }

    internal KafkaDependency(string identifier, int port, KafkaFlavor flavor)
    {
        Identifier = identifier;
        Port = port;
        Flavor = flavor;
    }

    public string ForFfi()
    {
        return ArenaJson.Serialize(new KafkaConfig
        {
            Type = Type,
            Identifier = Identifier,
            Port = Port,
            Flavor = Flavor == KafkaFlavor.Zookeeper ? "zookeeper" : "kraft",
        });
    }

    [JsonObject(ItemNullValueHandling = NullValueHandling.Ignore)]
    private sealed class KafkaConfig
    {
        [JsonProperty("type")] public string Type { get; set; } = default!;
        [JsonProperty("identifier")] public string Identifier { get; set; } = default!;
        [JsonProperty("port")] public int Port { get; set; }
        [JsonProperty("flavor")] public string? Flavor { get; set; }
    }
}

public sealed class KafkaDependencyBuilder
{
    private readonly string _name;
    private int _port = 9092;
    private KafkaFlavor _flavor = KafkaFlavor.Zookeeper;

    public KafkaDependencyBuilder(string name)
    {
        _name = name;
    }

    public KafkaDependencyBuilder WithPort(int port)
    {
        _port = port;
        return this;
    }

    public KafkaDependencyBuilder WithFlavor(KafkaFlavor flavor)
    {
        _flavor = flavor;
        return this;
    }

    public KafkaDependency Build()
    {
        var identifier = ArenaIdentifiers.Build("arena-kafka", _name);
        return new KafkaDependency(identifier, _port, _flavor);
    }
}
