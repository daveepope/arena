using System.Collections.Generic;
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
    public string? DatabaseName { get; }
    public string? DatabaseUsername { get; }
    public string? DatabasePassword { get; }
    public IReadOnlyList<string> StartupSqlScripts { get; }

    internal MssqlDependency(
        string identifier,
        int port,
        MssqlEncryption encryption,
        string? databaseName,
        string? databaseUsername,
        string? databasePassword,
        IReadOnlyList<string> startupSqlScripts)
    {
        Identifier = identifier;
        Port = port;
        Encryption = encryption;
        DatabaseName = databaseName;
        DatabaseUsername = databaseUsername;
        DatabasePassword = databasePassword;
        StartupSqlScripts = startupSqlScripts;
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
    private string? _databaseName;
    private string? _databaseUsername;
    private string? _databasePassword;
    private readonly List<string> _startupSqlScripts = new();

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

    public MssqlDependencyBuilder WithDatabaseName(string databaseName)
    {
        _databaseName = databaseName;
        return this;
    }

    public MssqlDependencyBuilder WithDatabaseUsername(string databaseUsername)
    {
        _databaseUsername = databaseUsername;
        return this;
    }

    public MssqlDependencyBuilder WithDatabasePassword(string databasePassword)
    {
        _databasePassword = databasePassword;
        return this;
    }

    public MssqlDependencyBuilder WithStartupSqlScripts(IEnumerable<string> scripts)
    {
        _startupSqlScripts.AddRange(scripts);
        return this;
    }

    public MssqlDependency Build()
    {
        var identifier = ArenaIdentifiers.Build("arena-mssql", _name);
        return new MssqlDependency(identifier, _port, _encryption, _databaseName, _databaseUsername, _databasePassword, _startupSqlScripts);
    }
}
