using System.Collections.Generic;
using ArenaDotnet.Xunit.Support;
using Newtonsoft.Json.Linq;

namespace ArenaDotnet.Xunit.Dep;

public sealed class HttpDependency : IArenaDependency
{
    public string Type => "http";
    public string Identifier { get; }
    public long? ExpirySeconds { get; internal set; }
    public int Port { get; }
    public string? ListenIp { get; }
    public string? ContainerName { get; }
    public string? ImageName { get; }
    public string? ImageTag { get; }
    public List<JToken>? Children => ChildrenWireFormat.Build(_children);

    private readonly IReadOnlyList<IArenaDependency> _children;

    internal HttpDependency(string identifier, int port, string? listenIp, string? containerName, string? imageName, string? imageTag, IReadOnlyList<IArenaDependency> children)
    {
        Identifier = identifier;
        Port = port;
        ListenIp = listenIp;
        ContainerName = containerName;
        ImageName = imageName;
        ImageTag = imageTag;
        _children = children;
    }

    public string ForFfi()
    {
        return ArenaJson.Serialize(this);
    }
}

public sealed class HttpDependencyBuilder
{
    private long? _expirySeconds;
    private readonly string _name;
    private int _port = 8080;
    private string? _listenIp;
    private string? _containerName;
    private string? _imageName;
    private string? _imageTag;
    private readonly List<IArenaDependency> _children = new();

    public HttpDependencyBuilder(string name)
    {
        _name = name;
    }

    public HttpDependencyBuilder WithPort(int port)
    {
        _port = port;
        return this;
    }

    public HttpDependencyBuilder WithListenIp(string listenIp)
    {
        _listenIp = listenIp;
        return this;
    }

    public HttpDependencyBuilder WithContainerName(string containerName)
    {
        _containerName = containerName;
        return this;
    }

    public HttpDependencyBuilder WithImageName(string imageName)
    {
        _imageName = imageName;
        return this;
    }

    public HttpDependencyBuilder WithImageTag(string imageTag)
    {
        _imageTag = imageTag;
        return this;
    }

    public HttpDependencyBuilder AddChildDependency(IArenaDependency child)
    {
        _children.Add(child);
        return this;
    }

    public HttpDependencyBuilder WithExpiry(System.TimeSpan expiry)
    {
        _expirySeconds = ExpirySeconds(expiry);
        return this;
    }

    public HttpDependencyBuilder WithoutExpiry()
    {
        _expirySeconds = 0;
        return this;
    }

    public HttpDependency Build()
    {
        var identifier = ArenaIdentifiers.Build("arena-http", _name);
        var built = new HttpDependency(identifier, _port, _listenIp, _containerName, _imageName, _imageTag, _children);
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
