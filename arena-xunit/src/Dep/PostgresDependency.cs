using ArenaXunit.Topology;
using ArenaXunit.Support;
using Newtonsoft.Json.Linq;

namespace ArenaXunit.Dep;

public sealed class PostgresDependency : IArenaMatchPiece
{
    private readonly JObject _config;
    public string Type => "postgres";
    public string Identifier => _config["identifier"]!.Value<string>();
    public int Port => (int)_config["port"]!;

    internal PostgresDependency(JObject config) => _config = config;

    public string ForFfi() => ArenaJson.Serialize(_config);
}

public sealed class PostgresDependencyBuilder
{
    private readonly JObject _config = ArenaJson.Object();

    public PostgresDependencyBuilder(string name)
    {
        _config["type"] = "postgres";
        _config["identifier"] = ArenaIdentifiers.Build("arena-postgres", name);
        _config["port"] = 5432;
    }

    public PostgresDependencyBuilder WithPort(int port) { _config["port"] = port; return this; }
    public PostgresDependency Build() => new PostgresDependency((JObject)_config.DeepClone());
}