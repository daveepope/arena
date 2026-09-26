using System.Collections.Generic;
using System.Linq;
using ArenaDotnet.Xunit.Support;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;

namespace ArenaDotnet.Xunit.Component;

public sealed class ContainerizedComponent : IArenaComponent
{
    public string Type => "container";
    public string Identifier { get; }
    public long? ExpirySeconds { get; internal set; }
    public string? Containerfile { get; }
    public string? Image { get; }
    public string? BuildContext { get; }
    public string? ImageTag { get; }
    public string? Platform { get; }
    public string? Network { get; }
    public IReadOnlyDictionary<string, string> EnvVars { get; }
    public IReadOnlyList<string> HostMappings { get; }

    private readonly List<RuntimeArgEntry> _runtimeArgs;
    private readonly List<PortMappingEntry> _portMappings;
    private readonly List<VolumeMappingEntry> _volumeMappings;
    private readonly List<ReadinessCheckEntry> _readinessChecks;
    private readonly List<IArenaComponent> _children;

    internal ContainerizedComponent(string identifier, string? containerfile, string? image, string? buildContext,
        string? imageTag, string? platform, string? network, Dictionary<string, string> envVars,
        List<RuntimeArgEntry> runtimeArgs, List<PortMappingEntry> portMappings,
        List<string> hostMappings, List<VolumeMappingEntry> volumeMappings,
        List<ReadinessCheckEntry> readinessChecks, List<IArenaComponent> children)
    {
        Identifier = identifier;
        Containerfile = containerfile;
        Image = image;
        BuildContext = buildContext;
        ImageTag = imageTag;
        Platform = platform;
        Network = network;
        EnvVars = envVars;
        _runtimeArgs = runtimeArgs;
        _portMappings = portMappings;
        HostMappings = hostMappings;
        _volumeMappings = volumeMappings;
        _readinessChecks = readinessChecks;
        _children = children;
    }

    public string ForFfi()
    {
        return ArenaJson.Serialize(new ContainerizedComponentConfig
        {
            Type = Type,
            Identifier = Identifier,
            ExpirySeconds = ExpirySeconds,
            Containerfile = Containerfile,
            Image = Image,
            BuildContext = BuildContext,
            ImageTag = ImageTag,
            Platform = Platform,
            Network = Network,
            EnvVars = EnvVars.ToDictionary(kv => kv.Key, kv => kv.Value),
            RuntimeArgs = RuntimeArgEntry.Build(_runtimeArgs),
            PortMappings = PortMappingEntry.Build(_portMappings),
            HostMappings = new List<string>(HostMappings),
            VolumeMappings = VolumeMappingEntry.Build(_volumeMappings),
            ReadinessChecks = ReadinessCheckWireFormat.Build(_readinessChecks),
            Children = ChildrenWireFormat.Build(_children),
        });
    }

    [JsonObject(ItemNullValueHandling = NullValueHandling.Ignore)]
    private sealed class ContainerizedComponentConfig
    {
        [JsonProperty("type")] public string Type { get; set; } = default!;
        [JsonProperty("identifier")] public string Identifier { get; set; } = default!;
        [JsonProperty("expiry_seconds", NullValueHandling = NullValueHandling.Ignore)] public long? ExpirySeconds { get; set; }
        [JsonProperty("containerfile")] public string? Containerfile { get; set; }
        [JsonProperty("image")] public string? Image { get; set; }
        [JsonProperty("build_context")] public string? BuildContext { get; set; }
        [JsonProperty("image_tag")] public string? ImageTag { get; set; }
        [JsonProperty("platform")] public string? Platform { get; set; }
        [JsonProperty("network")] public string? Network { get; set; }
        [JsonProperty("env_vars")] public Dictionary<string, string> EnvVars { get; set; } = default!;
        [JsonProperty("runtime_args")] public List<object> RuntimeArgs { get; set; } = default!;
        [JsonProperty("port_mappings")] public List<object> PortMappings { get; set; } = default!;
        [JsonProperty("host_mappings")] public List<string> HostMappings { get; set; } = default!;
        [JsonProperty("volume_mappings")] public List<object> VolumeMappings { get; set; } = default!;
        [JsonProperty("readiness_checks")] public List<object>? ReadinessChecks { get; set; }
        [JsonProperty("children")] public List<JToken>? Children { get; set; }
    }
}

internal sealed class VolumeMappingEntry
{
    public VolumeMappingEntry(string hostPath, string containerPath)
    {
        HostPath = hostPath;
        ContainerPath = containerPath;
    }

    public string HostPath { get; }
    public string ContainerPath { get; }

    public static List<object> Build(IReadOnlyList<VolumeMappingEntry> entries)
    {
        var result = new List<object>(entries.Count);
        foreach (var entry in entries)
            result.Add(new { host_path = entry.HostPath, container_path = entry.ContainerPath });
        return result;
    }
}

internal sealed class PortMappingEntry
{
    public PortMappingEntry(int hostPort, int containerPort)
    {
        HostPort = hostPort;
        ContainerPort = containerPort;
    }

    public int HostPort { get; }
    public int ContainerPort { get; }

    public static List<object> Build(IReadOnlyList<PortMappingEntry> entries)
    {
        var result = new List<object>(entries.Count);
        foreach (var entry in entries)
            result.Add(new { host_port = entry.HostPort, container_port = entry.ContainerPort });
        return result;
    }
}

public sealed class ContainerizedComponentBuilder
{
    private long? _expirySeconds;
    private readonly string _name;
    private string? _containerfile;
    private string? _image;
    private string? _buildContext;
    private string? _imageTag;
    private string? _platform;
    private string? _network;
    private readonly Dictionary<string, string> _envVars = new();
    private readonly List<RuntimeArgEntry> _runtimeArgs = new();
    private readonly List<PortMappingEntry> _portMappings = new();
    private readonly List<string> _hostMappings = new();
    private readonly List<VolumeMappingEntry> _volumeMappings = new();
    private readonly List<ReadinessCheckEntry> _readinessChecks = new();
    private readonly List<IArenaComponent> _children = new();

    public ContainerizedComponentBuilder(string name)
    {
        _name = name;
    }

    public static ContainerizedComponentBuilder FromImage(string name, string image)
    {
        var builder = new ContainerizedComponentBuilder(name);
        builder._image = image;
        return builder;
    }

    public ContainerizedComponentBuilder WithContainerfile(string path)
    {
        _containerfile = path;
        return this;
    }

    public ContainerizedComponentBuilder WithBuildContext(string path)
    {
        _buildContext = path;
        return this;
    }

    public ContainerizedComponentBuilder WithImageTag(string tag)
    {
        _imageTag = tag;
        return this;
    }

    public ContainerizedComponentBuilder WithPlatform(string platform)
    {
        _platform = platform;
        return this;
    }

    public ContainerizedComponentBuilder WithNetwork(string network)
    {
        _network = network;
        return this;
    }

    public ContainerizedComponentBuilder WithPortMapping(int hostPort, int containerPort)
    {
        _portMappings.Add(new PortMappingEntry(hostPort, containerPort));
        return this;
    }

    public ContainerizedComponentBuilder WithHostMapping(string hostMapping)
    {
        _hostMappings.Add(hostMapping);
        return this;
    }

    public ContainerizedComponentBuilder WithVolumeMapping(string hostPath, string containerPath)
    {
        _volumeMappings.Add(new VolumeMappingEntry(hostPath, containerPath));
        return this;
    }

    public ContainerizedComponentBuilder WithEnvVar(string key, string value)
    {
        _envVars[key] = value;
        return this;
    }

    public ContainerizedComponentBuilder WithRuntimeArg(string name, string value)
    {
        _runtimeArgs.Add(new RuntimeArgEntry(name, value));
        return this;
    }

    public ContainerizedComponentBuilder WithReadinessCheck(IArenaReadinessCheck check, string target)
    {
        return WithReadinessCheck(check, target, ReadinessCheckWireFormat.DefaultTimeoutMs);
    }

    public ContainerizedComponentBuilder WithReadinessCheck(IArenaReadinessCheck check, string target, long timeoutMs)
    {
        _readinessChecks.Add(new ReadinessCheckEntry(check, target, timeoutMs));
        return this;
    }

    public ContainerizedComponentBuilder AddChildComponent(IArenaComponent child)
    {
        _children.Add(child);
        return this;
    }

    public ContainerizedComponentBuilder WithExpiry(System.TimeSpan expiry)
    {
        _expirySeconds = ExpirySeconds(expiry);
        return this;
    }

    public ContainerizedComponentBuilder WithoutExpiry()
    {
        _expirySeconds = 0;
        return this;
    }

    public ContainerizedComponent Build()
    {
        var hasContainerfile = !string.IsNullOrEmpty(_containerfile);
        var hasImage = !string.IsNullOrEmpty(_image);
        if (hasContainerfile == hasImage)
            throw new System.InvalidOperationException(
                "exactly one of containerfile or image must be set");
        var identifier = ArenaIdentifiers.Build("arena-container", _name);
        var built = new ContainerizedComponent(identifier, _containerfile, _image, _buildContext, _imageTag,
            _platform, _network,
            new Dictionary<string, string>(_envVars), new List<RuntimeArgEntry>(_runtimeArgs),
            new List<PortMappingEntry>(_portMappings), new List<string>(_hostMappings),
            new List<VolumeMappingEntry>(_volumeMappings),
            new List<ReadinessCheckEntry>(_readinessChecks), new List<IArenaComponent>(_children));
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
