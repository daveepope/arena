using System.Collections.Generic;
using ArenaDotnet.Xunit.Support;
using Newtonsoft.Json.Linq;

namespace ArenaDotnet.Xunit.Dep;

public sealed class TemporalDependency : IArenaDependency
{
    public string Type => "temporal";
    public string Identifier { get; }
    public long? ExpirySeconds { get; internal set; }
    public int Port { get; }
    public int UiPort { get; }
    public string? Image { get; }
    public string? ImageName { get; }
    public string? ContainerName { get; }
    public List<JToken>? Children => ChildrenWireFormat.Build(_children);

    private readonly IReadOnlyList<IArenaDependency> _children;

    internal TemporalDependency(string identifier, int port, int uiPort, string? image, string? imageName, string? containerName, IReadOnlyList<IArenaDependency> children)
    {
        Identifier = identifier;
        Port = port;
        UiPort = uiPort;
        Image = string.IsNullOrEmpty(image) ? null : image;
        ImageName = string.IsNullOrEmpty(imageName) ? null : imageName;
        ContainerName = string.IsNullOrEmpty(containerName) ? null : containerName;
        _children = children;
    }

    public string ForFfi()
    {
        return ArenaJson.Serialize(this);
    }
}

public sealed class TemporalDependencyBuilder
{
    private long? _expirySeconds;
    private readonly string _name;
    private int _port = 7233;
    private int _uiPort = 8233;
    private string? _image;
    private string? _imageName;
    private string? _containerName;
    private readonly List<IArenaDependency> _children = new();

    public TemporalDependencyBuilder(string name)
    {
        _name = name;
    }

    public TemporalDependencyBuilder WithPort(int port)
    {
        _port = port;
        return this;
    }

    public TemporalDependencyBuilder WithUiPort(int uiPort)
    {
        _uiPort = uiPort;
        return this;
    }

    public TemporalDependencyBuilder WithImage(string image)
    {
        _image = image;
        return this;
    }

    public TemporalDependencyBuilder WithImageName(string imageName)
    {
        _imageName = imageName;
        return this;
    }

    public TemporalDependencyBuilder WithContainerName(string containerName)
    {
        _containerName = containerName;
        return this;
    }

    public TemporalDependencyBuilder AddChildDependency(IArenaDependency child)
    {
        _children.Add(child);
        return this;
    }

    public TemporalDependencyBuilder WithExpiry(System.TimeSpan expiry)
    {
        _expirySeconds = ExpirySeconds(expiry);
        return this;
    }

    public TemporalDependencyBuilder WithoutExpiry()
    {
        _expirySeconds = 0;
        return this;
    }

    public TemporalDependency Build()
    {
        var identifier = ArenaIdentifiers.Build("arena-temporal", _name);
        var built = new TemporalDependency(identifier, _port, _uiPort, _image, _imageName, _containerName, _children);
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
