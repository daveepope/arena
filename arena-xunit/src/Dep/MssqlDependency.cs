using ArenaXunit.Support;

namespace ArenaXunit.Dep;

public enum MssqlEncryption
{
    Off,
    On
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
        return ArenaJson.Serialize(this);
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
