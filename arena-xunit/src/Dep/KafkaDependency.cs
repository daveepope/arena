using System.Collections.Generic;
using ArenaDotnet.Xunit.Support;
using Newtonsoft.Json.Linq;

namespace ArenaDotnet.Xunit.Dep;

public enum KafkaFlavor
{
    ApacheNative,
    Confluent
}

public sealed class KafkaDependency : IArenaDependency
{
    public string Type => "kafka";
    public string Identifier { get; }
    public int Port { get; }
    public KafkaFlavor Flavor { get; }
    public List<JToken>? Children => ChildrenWireFormat.Build(_children);

    private readonly IReadOnlyList<IArenaDependency> _children;

    internal KafkaDependency(string identifier, int port, KafkaFlavor flavor, IReadOnlyList<IArenaDependency> children)
    {
        Identifier = identifier;
        Port = port;
        Flavor = flavor;
        _children = children;
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
    private readonly List<IArenaDependency> _children = new();

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

    public KafkaDependencyBuilder AddChildDependency(IArenaDependency child)
    {
        _children.Add(child);
        return this;
    }

    public KafkaDependency Build()
    {
        var identifier = ArenaIdentifiers.Build("arena-kafka", _name);
        return new KafkaDependency(identifier, _port, _flavor, _children);
    }
}
