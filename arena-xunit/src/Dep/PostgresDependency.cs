using System.Collections.Generic;
using ArenaDotnet.Xunit.Support;

namespace ArenaDotnet.Xunit.Dep;

public sealed class PostgresDependency : IArenaMatchPiece
{
    public string Type => "postgres";
    public string Identifier { get; }
    public int Port { get; }
    public string? DatabaseName { get; }
    public string? DatabaseUsername { get; }
    public string? DatabasePassword { get; }
    public IReadOnlyList<string> StartupSqlScripts { get; }

    internal PostgresDependency(
        string identifier,
        int port,
        string? databaseName,
        string? databaseUsername,
        string? databasePassword,
        IReadOnlyList<string> startupSqlScripts)
    {
        Identifier = identifier;
        Port = port;
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

public sealed class PostgresDependencyBuilder
{
    private readonly string _name;
    private int _port = 5432;
    private string? _databaseName;
    private string? _databaseUsername;
    private string? _databasePassword;
    private readonly List<string> _startupSqlScripts = new();

    public PostgresDependencyBuilder(string name)
    {
        _name = name;
    }

    public PostgresDependencyBuilder WithPort(int port)
    {
        _port = port;
        return this;
    }

    public PostgresDependencyBuilder WithDatabaseName(string databaseName)
    {
        _databaseName = databaseName;
        return this;
    }

    public PostgresDependencyBuilder WithDatabaseUsername(string databaseUsername)
    {
        _databaseUsername = databaseUsername;
        return this;
    }

    public PostgresDependencyBuilder WithDatabasePassword(string databasePassword)
    {
        _databasePassword = databasePassword;
        return this;
    }

    public PostgresDependencyBuilder WithStartupSqlScripts(IEnumerable<string> scripts)
    {
        _startupSqlScripts.AddRange(scripts);
        return this;
    }

    public PostgresDependency Build()
    {
        var identifier = ArenaIdentifiers.Build("arena-postgres", _name);
        return new PostgresDependency(identifier, _port, _databaseName, _databaseUsername, _databasePassword, _startupSqlScripts);
    }
}
