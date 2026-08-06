using ArenaDotnet.Xunit.Support;

namespace ArenaDotnet.Xunit.Dep;

public enum KafkaFlavor
{
    ApacheNative,
    Confluent
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
        return ArenaJson.Serialize(this);
    }
}

public sealed class KafkaDependencyBuilder
{
    private readonly string _name;
    private int _port = 9092;
    private KafkaFlavor _flavor = KafkaFlavor.ApacheNative;

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
