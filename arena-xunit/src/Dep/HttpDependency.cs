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

    internal HttpDependency(string identifier, int port, string? listenIp)
    {
        Identifier = identifier;
        Port = port;
        ListenIp = listenIp;
    }

    public string ForFfi()
    {
        return ArenaJson.Serialize(new HttpConfig
        {
            Type = Type,
            Identifier = Identifier,
            Port = Port,
            ListenIp = ListenIp,
        });
    }

    [JsonObject(ItemNullValueHandling = NullValueHandling.Ignore)]
    private sealed class HttpConfig
    {
        [JsonProperty("type")] public string Type { get; set; } = default!;
        [JsonProperty("identifier")] public string Identifier { get; set; } = default!;
        [JsonProperty("port")] public int Port { get; set; }
        [JsonProperty("listen_ip")] public string? ListenIp { get; set; }
    }
}

public sealed class HttpDependencyBuilder
{
    private readonly string _name;
    private int _port = 8080;
    private string? _listenIp;

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

    public HttpDependency Build()
    {
        var identifier = ArenaIdentifiers.Build("arena-http", _name);
        return new HttpDependency(identifier, _port, _listenIp);
    }
}
