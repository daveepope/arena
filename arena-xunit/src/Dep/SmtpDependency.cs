using ArenaXunit.Support;

namespace ArenaXunit.Dep;

public sealed class SmtpDependency : IArenaMatchPiece
{
    public string Type => "smtp";
    public string Identifier { get; }
    public int Port { get; }
    public int UiPort { get; }
    public string? TlsMode { get; }
    public string? Image { get; }
    public string? ContainerName { get; }

    internal SmtpDependency(string identifier, int port, int uiPort, string? tlsMode, string? image, string? containerName)
    {
        Identifier = identifier;
        Port = port;
        UiPort = uiPort;
        TlsMode = string.IsNullOrEmpty(tlsMode) ? null : tlsMode;
        Image = string.IsNullOrEmpty(image) ? null : image;
        ContainerName = string.IsNullOrEmpty(containerName) ? null : containerName;
    }

    public string ForFfi()
    {
        return ArenaJson.Serialize(this);
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
