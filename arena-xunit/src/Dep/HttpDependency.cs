using ArenaXunit.Topology;
using ArenaXunit.Support;
using Newtonsoft.Json;

namespace ArenaXunit.Dep;

public sealed class HttpDependency : IArenaMatchPiece
{
    public string Type => "http";
    public string Identifier { get; }
    public int Port { get; }
    public string? ListenIp { get; }
    public string? ContainerName { get; }
    public string? ImageName { get; }
    public string? ImageTag { get; }

    internal HttpDependency(string identifier, int port, string? listenIp, string? containerName, string? imageName, string? imageTag)
    {
        Identifier = identifier;
        Port = port;
        ListenIp = listenIp;
        ContainerName = containerName;
        ImageName = imageName;
        ImageTag = imageTag;
    }

    public string ForFfi()
    {
        return ArenaJson.Serialize(new HttpConfig
        {
            Type = Type,
            Identifier = Identifier,
            Port = Port,
            ListenIp = ListenIp,
            ContainerName = ContainerName,
            ImageName = ImageName,
            ImageTag = ImageTag,
        });
    }

    [JsonObject(ItemNullValueHandling = NullValueHandling.Ignore)]
    private sealed class HttpConfig
    {
        [JsonProperty("type")] public string Type { get; set; } = default!;
        [JsonProperty("identifier")] public string Identifier { get; set; } = default!;
        [JsonProperty("port")] public int Port { get; set; }
        [JsonProperty("listen_ip")] public string? ListenIp { get; set; }
        [JsonProperty("container_name")] public string? ContainerName { get; set; }
        [JsonProperty("image_name")] public string? ImageName { get; set; }
        [JsonProperty("image_tag")] public string? ImageTag { get; set; }
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

    public HttpDependency Build()
    {
        var identifier = ArenaIdentifiers.Build("arena-http", _name);
        return new HttpDependency(identifier, _port, _listenIp, _containerName, _imageName, _imageTag);
    }
}
