using ArenaXunit.Topology;
using ArenaXunit.Support;
using Newtonsoft.Json;

namespace ArenaXunit.Dep;

public sealed class SmtpDependency : IArenaMatchPiece
{
    public string Type => "smtp";
    public string Identifier { get; }
    public int Port { get; }
    public int UiPort { get; }

    private readonly string? _tlsMode;
    private readonly string? _image;
    private readonly string? _containerName;

    internal SmtpDependency(string identifier, int port, int uiPort, string? tlsMode, string? image, string? containerName)
    {
        Identifier = identifier;
        Port = port;
        UiPort = uiPort;
        _tlsMode = tlsMode;
        _image = image;
        _containerName = containerName;
    }

    public string ForFfi()
    {
        var config = new SmtpConfig
        {
            Type = Type,
            Identifier = Identifier,
            Port = Port,
            UiPort = UiPort,
        };

        if (!string.IsNullOrEmpty(_tlsMode)) config.TlsMode = _tlsMode;
        if (!string.IsNullOrEmpty(_image)) config.Image = _image;
        if (!string.IsNullOrEmpty(_containerName)) config.ContainerName = _containerName;

        return ArenaJson.Serialize(config);
    }

    [JsonObject(ItemNullValueHandling = NullValueHandling.Ignore)]
    private sealed class SmtpConfig
    {
        [JsonProperty("type")] public string Type { get; set; } = default!;
        [JsonProperty("identifier")] public string Identifier { get; set; } = default!;
        [JsonProperty("port")] public int Port { get; set; }
        [JsonProperty("ui_port")] public int UiPort { get; set; }
        [JsonProperty("tls_mode")] public string? TlsMode { get; set; }
        [JsonProperty("image")] public string? Image { get; set; }
        [JsonProperty("container_name")] public string? ContainerName { get; set; }
    }
}

public sealed class SmtpDependencyBuilder
{
    private readonly string _name;
    private int _port = 1025;
    private int _uiPort = 8025;
    private string? _tlsMode;
    private string? _image;
    private string? _containerName;

    public SmtpDependencyBuilder(string name)
    {
        _name = name;
    }

    public SmtpDependencyBuilder WithPort(int port)
    {
        _port = port;
        return this;
    }

    public SmtpDependencyBuilder WithUiPort(int uiPort)
    {
        _uiPort = uiPort;
        return this;
    }

    public SmtpDependencyBuilder WithStarttls()
    {
        _tlsMode = "starttls";
        return this;
    }

    public SmtpDependencyBuilder WithImplicitTls()
    {
        _tlsMode = "implicit";
        return this;
    }

    public SmtpDependencyBuilder WithImage(string image)
    {
        _image = image;
        return this;
    }

    public SmtpDependencyBuilder WithContainerName(string containerName)
    {
        _containerName = containerName;
        return this;
    }

    public SmtpDependency Build()
    {
        var identifier = ArenaIdentifiers.Build("arena-smtp", _name);
        return new SmtpDependency(identifier, _port, _uiPort, _tlsMode, _image, _containerName);
    }
}
