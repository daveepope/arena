using ArenaXunit.Match;
using ArenaXunit.Support;
using Newtonsoft.Json;

namespace ArenaXunit.Dep;

public enum MssqlEncryption
{
    Off,
    On,
    Strict
}

public sealed class MssqlDependency : IArenaMatchPiece
{
    public string Type => "mssql";
    public string Identifier { get; }
    public int Port { get; }
    public MssqlEncryption Encryption { get; }

    internal MssqlDependency(string identifier, int port, MssqlEncryption encryption)
    {
        Identifier = identifier;
        Port = port;
        Encryption = encryption;
    }

    public string ForFfi()
    {
        return ArenaJson.Serialize(new MssqlConfig
        {
            Type = Type,
            Identifier = Identifier,
            Port = Port,
            Encryption = Encryption switch
            {
                MssqlEncryption.Off => "off",
                MssqlEncryption.On => "on",
                MssqlEncryption.Strict => "strict",
                _ => "off"
            },
        });
    }

    [JsonObject(ItemNullValueHandling = NullValueHandling.Ignore)]
    private sealed class MssqlConfig
    {
        [JsonProperty("type")] public string Type { get; set; } = default!;
        [JsonProperty("identifier")] public string Identifier { get; set; } = default!;
        [JsonProperty("port")] public int Port { get; set; }
        [JsonProperty("encryption")] public string? Encryption { get; set; }
    }
}

public sealed class MssqlDependencyBuilder
{
    private readonly string _name;
    private int _port = 1433;
    private MssqlEncryption _encryption = MssqlEncryption.On;

    public MssqlDependencyBuilder(string name)
    {
        _name = name;
    }

    public MssqlDependencyBuilder WithPort(int port)
    {
        _port = port;
        return this;
    }

    public MssqlDependencyBuilder WithEncryption(MssqlEncryption encryption)
    {
        _encryption = encryption;
        return this;
    }

    public MssqlDependency Build()
    {
        var identifier = ArenaIdentifiers.Build("arena-mssql", _name);
        return new MssqlDependency(identifier, _port, _encryption);
    }
}
