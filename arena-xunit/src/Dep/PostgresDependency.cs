using System.Collections.Generic;
using ArenaDotnet.Xunit.Support;
using Newtonsoft.Json.Linq;

namespace ArenaDotnet.Xunit.Dep;

public sealed class PostgresDependency : IArenaDependency
{
    public string Type => "postgres";
    public string Identifier { get; }
    public long? ExpirySeconds { get; internal set; }
    public int Port { get; }
    public string? DatabaseName { get; }
    public string? DatabaseUsername { get; }
    public string? DatabasePassword { get; }
    public string? ImageName { get; }
    public string? Image { get; }
    public string? ContainerName { get; }
    public IReadOnlyList<string> StartupSqlScripts { get; }
    public List<JToken>? Children => ChildrenWireFormat.Build(_children);

    private readonly IReadOnlyList<IArenaDependency> _children;

    internal PostgresDependency(
        string identifier,
        int port,
        string? databaseName,
        string? databaseUsername,
        string? databasePassword,
        string? imageName,
        string? image,
        string? containerName,
        IReadOnlyList<string> startupSqlScripts,
        IReadOnlyList<IArenaDependency> children)
    {
        Identifier = identifier;
        Port = port;
        DatabaseName = databaseName;
        DatabaseUsername = databaseUsername;
        DatabasePassword = databasePassword;
        ImageName = imageName;
        Image = image;
        ContainerName = containerName;
        StartupSqlScripts = startupSqlScripts;
        _children = children;
    }

    public string ForFfi()
    {
        return ArenaJson.Serialize(this);
    }
}

public sealed class PostgresDependencyBuilder
{
    private long? _expirySeconds;
    private readonly string _name;
    private int _port = 5432;
    private string? _databaseName;
    private string? _databaseUsername;
    private string? _databasePassword;
    private string? _imageName;
    private string? _image;
    private string? _containerName;
    private readonly List<string> _startupSqlScripts = new();
    private readonly List<IArenaDependency> _children = new();

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

    public PostgresDependencyBuilder WithImageName(string imageName)
    {
        _imageName = imageName;
        return this;
    }

    public PostgresDependencyBuilder WithImage(string image)
    {
        _image = image;
        return this;
    }

    public PostgresDependencyBuilder WithContainerName(string containerName)
    {
        _containerName = containerName;
        return this;
    }

    public PostgresDependencyBuilder WithStartupSqlScripts(IEnumerable<string> scripts)
    {
        _startupSqlScripts.AddRange(scripts);
        return this;
    }

    public PostgresDependencyBuilder AddChildDependency(IArenaDependency child)
    {
        _children.Add(child);
        return this;
    }

    public PostgresDependencyBuilder WithExpiry(System.TimeSpan expiry)
    {
        _expirySeconds = ExpirySeconds(expiry);
        return this;
    }

    public PostgresDependencyBuilder WithoutExpiry()
    {
        _expirySeconds = 0;
        return this;
    }

    public PostgresDependency Build()
    {
        var identifier = ArenaIdentifiers.Build("arena-postgres", _name);
        var built = new PostgresDependency(identifier, _port, _databaseName, _databaseUsername, _databasePassword, _imageName, _image, _containerName, _startupSqlScripts, _children);
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
