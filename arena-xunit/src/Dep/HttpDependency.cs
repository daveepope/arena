using System.Collections.Generic;
using ArenaDotnet.Xunit.Support;
using Newtonsoft.Json.Linq;

namespace ArenaDotnet.Xunit.Dep;

public sealed class HttpDependency : IArenaMatchPiece
{
    public string Type => "http";
    public string Identifier { get; }
    public int Port { get; }
    public string? ListenIp { get; }
    public string? ContainerName { get; }
    public string? ImageName { get; }
    public string? ImageTag { get; }
    public List<JToken>? Children => ChildrenWireFormat.Build(_children);

    private readonly IReadOnlyList<IArenaMatchPiece> _children;

    internal HttpDependency(string identifier, int port, string? listenIp, string? containerName, string? imageName, string? imageTag, IReadOnlyList<IArenaMatchPiece> children)
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
    private readonly string _name;
    private int _port = 8080;
    private string? _listenIp;
    private string? _containerName;
    private string? _imageName;
    private string? _imageTag;
    private readonly List<IArenaMatchPiece> _children = new();

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

    public HttpDependencyBuilder WithChildDependencies(IEnumerable<IArenaMatchPiece> children)
    {
        _children.AddRange(children);
        return this;
    }

    public HttpDependency Build()
    {
        var identifier = ArenaIdentifiers.Build("arena-http", _name);
        return new HttpDependency(identifier, _port, _listenIp, _containerName, _imageName, _imageTag, _children);
    }
}
