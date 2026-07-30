using ArenaXunit.Topology;
using ArenaXunit.Support;
using Newtonsoft.Json.Linq;

namespace ArenaXunit.Dep;

public sealed class HttpDependency : IArenaMatchPiece
{
    private readonly JObject _config;
    public string Type => "http";
    public string Identifier => _config["identifier"]!.Value<string>();
    public int Port => (int)_config["port"]!;
    public string? ListenIp => (string?)_config["listen_ip"];
    public string? ContainerName => (string?)_config["container_name"];
    public string? ImageName => (string?)_config["image_name"];
    public string? ImageTag => (string?)_config["image_tag"];

    internal HttpDependency(JObject config) => _config = config;

    public string ForFfi() => ArenaJson.Serialize(_config);
}

public sealed class HttpDependencyBuilder
{
    private readonly JObject _config = ArenaJson.Object();

    public HttpDependencyBuilder(string name)
    {
        _config["type"] = "http";
        _config["identifier"] = ArenaIdentifiers.Build("arena-http", name);
        _config["port"] = 8080;
    }

    public HttpDependencyBuilder WithPort(int port) { _config["port"] = port; return this; }
    public HttpDependencyBuilder WithListenIp(string v) { _config["listen_ip"] = v; return this; }
    public HttpDependencyBuilder WithContainerName(string v) { _config["container_name"] = v; return this; }
    public HttpDependencyBuilder WithImageName(string v) { _config["image_name"] = v; return this; }
    public HttpDependencyBuilder WithImageTag(string v) { _config["image_tag"] = v; return this; }
    public HttpDependency Build() => new HttpDependency((JObject)_config.DeepClone());
}