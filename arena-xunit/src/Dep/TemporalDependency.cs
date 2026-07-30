using ArenaXunit.Topology;
using ArenaXunit.Support;
using Newtonsoft.Json.Linq;

namespace ArenaXunit.Dep;

public sealed class TemporalDependency : IArenaMatchPiece
{
    private readonly JObject _config;
    public string Type => "temporal";
    public string Identifier => _config["identifier"]!.Value<string>();
    public int Port => (int)_config["port"]!;

    internal TemporalDependency(JObject config) => _config = config;

    public string ForFfi() => ArenaJson.Serialize(_config);
}

public sealed class TemporalDependencyBuilder
{
    private readonly JObject _config = ArenaJson.Object();

    public TemporalDependencyBuilder(string name)
    {
        _config["type"] = "temporal";
        _config["identifier"] = ArenaIdentifiers.Build("arena-temporal", name);
        _config["port"] = 7233;
    }

    public TemporalDependencyBuilder WithPort(int port) { _config["port"] = port; return this; }
    public TemporalDependency Build() => new TemporalDependency((JObject)_config.DeepClone());
}
