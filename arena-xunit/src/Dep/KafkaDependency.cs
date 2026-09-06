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
    public long? ExpirySeconds { get; internal set; }
    public int Port { get; }
    public KafkaFlavor Flavor { get; }
    public string? ImageName { get; }
    public string? ContainerName { get; }
    public IReadOnlyList<string>? Topics { get; }
    public List<JToken>? Children => ChildrenWireFormat.Build(_children);

    private readonly IReadOnlyList<IArenaDependency> _children;

    internal KafkaDependency(
        string identifier,
        int port,
        KafkaFlavor flavor,
        string? imageName,
        string? containerName,
        IReadOnlyList<string> topics,
        IReadOnlyList<IArenaDependency> children)
    {
        Identifier = identifier;
        Port = port;
        Flavor = flavor;
        ImageName = imageName;
        ContainerName = containerName;
        Topics = topics.Count > 0 ? topics : null;
        _children = children;
    }

    public string ForFfi()
    {
        return ArenaJson.Serialize(this);
    }
}

public sealed class KafkaDependencyBuilder
{
    private long? _expirySeconds;
    private readonly string _name;
    private int _port = 9092;
    private KafkaFlavor _flavor = KafkaFlavor.ApacheNative;
    private string? _imageName;
    private string? _containerName;
    private readonly List<string> _topics = new();
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

    public KafkaDependencyBuilder WithImageName(string imageName)
    {
        _imageName = imageName;
        return this;
    }

    public KafkaDependencyBuilder WithContainerName(string containerName)
    {
        _containerName = containerName;
        return this;
    }

    public KafkaDependencyBuilder WithTopic(string topic)
    {
        _topics.Add(topic);
        return this;
    }

    public KafkaDependencyBuilder AddChildDependency(IArenaDependency child)
    {
        _children.Add(child);
        return this;
    }

    public KafkaDependencyBuilder WithExpiry(System.TimeSpan expiry)
    {
        _expirySeconds = ExpirySeconds(expiry);
        return this;
    }

    public KafkaDependencyBuilder WithoutExpiry()
    {
        _expirySeconds = 0;
        return this;
    }

    public KafkaDependency Build()
    {
        var identifier = ArenaIdentifiers.Build("arena-kafka", _name);
        var built = new KafkaDependency(identifier, _port, _flavor, _imageName, _containerName, new List<string>(_topics), _children);
        built.ExpirySeconds = _expirySeconds;
        return built;
    }

    private static long ExpirySeconds(System.TimeSpan expiry)
    {
        if (expiry < System.TimeSpan.Zero)
            throw new System.ArgumentOutOfRangeException(nameof(expiry), "expiry must not be negative");
        var seconds = (long)expiry.TotalSeconds;
        return seconds == 0 && expiry > System.TimeSpan.Zero ? 1 : seconds;
    }

}
