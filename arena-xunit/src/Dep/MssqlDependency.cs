using ArenaXunit.Topology;
using ArenaXunit.Support;
using Newtonsoft.Json.Linq;

namespace ArenaXunit.Dep;

public enum MssqlEncryption
{
    Off,
    On,
    Strict
}

public sealed class MssqlDependency : IArenaMatchPiece
{
    private readonly JObject _config;
    public string Type => "mssql";
    public string Identifier => _config["identifier"]!.Value<string>();
    public int Port => (int)_config["port"]!;
    public MssqlEncryption Encryption => _config["encryption"]!.Value<string>() switch
    {
        "off" => MssqlEncryption.Off,
        "on" => MssqlEncryption.On,
        "strict" => MssqlEncryption.Strict,
        _ => MssqlEncryption.Off
    };

    internal MssqlDependency(JObject config) => _config = config;

    public string ForFfi() => ArenaJson.Serialize(_config);
}

public sealed class MssqlDependencyBuilder
{
    private readonly JObject _config = ArenaJson.Object();

    public MssqlDependencyBuilder(string name)
    {
        _config["type"] = "mssql";
        _config["identifier"] = ArenaIdentifiers.Build("arena-mssql", name);
        _config["port"] = 1433;
        _config["encryption"] = "on";
    }

    public MssqlDependencyBuilder WithPort(int port) { _config["port"] = port; return this; }
    public MssqlDependencyBuilder WithEncryption(MssqlEncryption encryption)
    {
        _config["encryption"] = encryption switch
        {
            MssqlEncryption.Off => "off",
            MssqlEncryption.On => "on",
            MssqlEncryption.Strict => "strict",
            _ => "off"
        };
        return this;
    }
    public MssqlDependency Build() => new MssqlDependency((JObject)_config.DeepClone());
}