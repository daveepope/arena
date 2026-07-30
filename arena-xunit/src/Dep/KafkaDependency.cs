using ArenaXunit.Topology;
using ArenaXunit.Support;
using Newtonsoft.Json.Linq;

namespace ArenaXunit.Dep;

public enum KafkaFlavor
{
    Zookeeper,
    KRaft
}

public sealed class KafkaDependency : IArenaMatchPiece
{
    private readonly JObject _config;
    public string Type => "kafka";
    public string Identifier => _config["identifier"]!.Value<string>();
    public int Port => (int)_config["port"]!;
    public KafkaFlavor Flavor => _config["flavor"]!.Value<string>() == "zookeeper" ? KafkaFlavor.Zookeeper : KafkaFlavor.KRaft;

    internal KafkaDependency(JObject config) => _config = config;

    public string ForFfi() => ArenaJson.Serialize(_config);
}

public sealed class KafkaDependencyBuilder
{
    private readonly JObject _config = ArenaJson.Object();

    public KafkaDependencyBuilder(string name)
    {
        _config["type"] = "kafka";
        _config["identifier"] = ArenaIdentifiers.Build("arena-kafka", name);
        _config["port"] = 9092;
        _config["flavor"] = "zookeeper";
    }

    public KafkaDependencyBuilder WithPort(int port) { _config["port"] = port; return this; }
    public KafkaDependencyBuilder WithFlavor(KafkaFlavor flavor) { _config["flavor"] = flavor == KafkaFlavor.Zookeeper ? "zookeeper" : "kraft"; return this; }
    public KafkaDependency Build() => new KafkaDependency((JObject)_config.DeepClone());
}