using System;
using System.Collections.Generic;
using ArenaDotnet.Xunit.Support;
using Newtonsoft.Json.Linq;

namespace ArenaDotnet.Xunit.Dep;

public sealed class OracleDependency : IArenaDependency
{
    public string Type => "oracledb";
    public string Identifier { get; }
    public long? ExpirySeconds { get; internal set; }
    public int Port { get; }
    public string? DatabaseName { get; }
    public string? DatabaseUsername { get; }
    public string? DatabasePassword { get; }
    public string? AdminPassword { get; }
    public string? ImageName { get; }
    public string? Image { get; }
    public string? ContainerName { get; }
    public IReadOnlyList<string> StartupSqlScripts { get; }
    public string? SetupMode { get; }
    public long? SqlReadinessTimeoutMs { get; }
    public List<JToken>? Children => ChildrenWireFormat.Build(_children);

    private readonly IReadOnlyList<IArenaDependency> _children;

    internal OracleDependency(
        string identifier,
        int port,
        string? databaseName,
        string? databaseUsername,
        string? databasePassword,
        string? adminPassword,
        string? imageName,
        string? image,
        string? containerName,
        IReadOnlyList<string> startupSqlScripts,
        string? setupMode,
        long? sqlReadinessTimeoutMs,
        IReadOnlyList<IArenaDependency> children)
    {
        Identifier = identifier;
        Port = port;
        DatabaseName = databaseName;
        DatabaseUsername = databaseUsername;
        DatabasePassword = databasePassword;
        AdminPassword = adminPassword;
        ImageName = imageName;
        Image = image;
        ContainerName = containerName;
        StartupSqlScripts = startupSqlScripts;
        SetupMode = setupMode;
        SqlReadinessTimeoutMs = sqlReadinessTimeoutMs;
        _children = children;
    }

    public string ForFfi()
    {
        return ArenaJson.Serialize(this);
    }
}

public sealed class OracleDependencyBuilder
{
    private long? _expirySeconds;
    private readonly string _name;
    private int _port = 1521;
    private string? _databaseName;
    private string? _databaseUsername;
    private string? _databasePassword;
    private string? _adminPassword;
    private string? _imageName;
    private string? _image;
    private string? _containerName;
    private readonly List<string> _startupSqlScripts = new();
    private string? _setupMode;
    private long? _sqlReadinessTimeoutMs;
    private readonly List<IArenaDependency> _children = new();

    public OracleDependencyBuilder(string name)
    {
        _name = name;
    }

    public OracleDependencyBuilder WithPort(int port)
    {
        _port = port;
        return this;
    }

    public OracleDependencyBuilder WithDatabaseName(string databaseName)
    {
        _databaseName = databaseName;
        return this;
    }

    public OracleDependencyBuilder WithDatabaseUsername(string databaseUsername)
    {
        _databaseUsername = databaseUsername;
        return this;
    }

    public OracleDependencyBuilder WithDatabasePassword(string databasePassword)
    {
        _databasePassword = databasePassword;
        return this;
    }

    public OracleDependencyBuilder WithAdminPassword(string adminPassword)
    {
        _adminPassword = adminPassword;
        return this;
    }

    public OracleDependencyBuilder WithImageName(string imageName)
    {
        _imageName = imageName;
        return this;
    }

    public OracleDependencyBuilder WithImage(string image)
    {
        _image = image;
        return this;
    }

    public OracleDependencyBuilder WithContainerName(string containerName)
    {
        _containerName = containerName;
        return this;
    }

    public OracleDependencyBuilder WithStartupSqlScripts(IEnumerable<string> scripts)
    {
        _startupSqlScripts.AddRange(scripts);
        return this;
    }

    public OracleDependencyBuilder FullBuild()
    {
        _setupMode = "full_build";
        return this;
    }

    public OracleDependencyBuilder WithSqlReadinessTimeout(TimeSpan timeout)
    {
        _sqlReadinessTimeoutMs = (long)timeout.TotalMilliseconds;
        return this;
    }

    public OracleDependencyBuilder AddChildDependency(IArenaDependency child)
    {
        _children.Add(child);
        return this;
    }

    public OracleDependencyBuilder WithExpiry(System.TimeSpan expiry)
    {
        _expirySeconds = ExpirySeconds(expiry);
        return this;
    }

    public OracleDependencyBuilder WithoutExpiry()
    {
        _expirySeconds = 0;
        return this;
    }

    public OracleDependency Build()
    {
        var identifier = ArenaIdentifiers.Build("arena-oracle", _name);
        var built = new OracleDependency(identifier, _port, _databaseName, _databaseUsername, _databasePassword, _adminPassword, _imageName, _image, _containerName, _startupSqlScripts, _setupMode, _sqlReadinessTimeoutMs, _children);
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
