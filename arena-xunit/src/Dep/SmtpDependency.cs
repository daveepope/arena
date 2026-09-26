using System.Collections.Generic;
using ArenaDotnet.Xunit.Support;
using Newtonsoft.Json.Linq;

namespace ArenaDotnet.Xunit.Dep;

public sealed class SmtpDependency : IArenaDependency
{
    public string Type => "smtp";
    public string Identifier { get; }
    public long? ExpirySeconds { get; internal set; }
    public int Port { get; }
    public int UiPort { get; }
    public string? TlsMode { get; }
    public string? ImageName { get; }
    public string? Image { get; }
    public string? ContainerName { get; }
    public List<JToken>? Children => ChildrenWireFormat.Build(_children);

    private readonly IReadOnlyList<IArenaDependency> _children;

    internal SmtpDependency(string identifier, int port, int uiPort, string? tlsMode, string? imageName, string? image, string? containerName, IReadOnlyList<IArenaDependency> children)
    {
        Identifier = identifier;
        Port = port;
        UiPort = uiPort;
        TlsMode = string.IsNullOrEmpty(tlsMode) ? null : tlsMode;
        ImageName = string.IsNullOrEmpty(imageName) ? null : imageName;
        Image = string.IsNullOrEmpty(image) ? null : image;
        ContainerName = string.IsNullOrEmpty(containerName) ? null : containerName;
        _children = children;
    }

    public string ForFfi()
    {
        return ArenaJson.Serialize(this);
    }
}

public sealed class SmtpDependencyBuilder
{
    private long? _expirySeconds;
    private readonly string _name;
    private int _port = 1025;
    private int _uiPort = 8025;
    private string? _tlsMode;
    private string? _imageName;
    private string? _image;
    private string? _containerName;
    private readonly List<IArenaDependency> _children = new();

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

    public SmtpDependencyBuilder WithImageName(string imageName)
    {
        _imageName = imageName;
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

    public SmtpDependencyBuilder AddChildDependency(IArenaDependency child)
    {
        _children.Add(child);
        return this;
    }

    public SmtpDependencyBuilder WithExpiry(System.TimeSpan expiry)
    {
        _expirySeconds = ExpirySeconds(expiry);
        return this;
    }

    public SmtpDependencyBuilder WithoutExpiry()
    {
        _expirySeconds = 0;
        return this;
    }

    public SmtpDependency Build()
    {
        var identifier = ArenaIdentifiers.Build("arena-smtp", _name);
        var built = new SmtpDependency(identifier, _port, _uiPort, _tlsMode, _imageName, _image, _containerName, _children);
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
