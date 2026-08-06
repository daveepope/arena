using System.Collections.Generic;
using System.Linq;
using ArenaDotnet.Xunit.Support;
using Newtonsoft.Json;

namespace ArenaDotnet.Xunit.Component;

public sealed class ContainerizedComponent : IArenaMatchPiece
{
    public string Type => "container";
    public string Identifier { get; }
    public string Containerfile { get; }
    public string? BuildContext { get; }
    public string? ImageTag { get; }
    public string? Network { get; }
    public IReadOnlyDictionary<string, string> EnvVars { get; }
    public IReadOnlyList<string> HostMappings { get; }

    private readonly List<RuntimeArgEntry> _runtimeArgs;
    private readonly List<PortMappingEntry> _portMappings;
    private readonly List<ReadinessCheckEntry> _readinessChecks;

    internal ContainerizedComponent(string identifier, string containerfile, string? buildContext,
        string? imageTag, string? network, Dictionary<string, string> envVars,
        List<RuntimeArgEntry> runtimeArgs, List<PortMappingEntry> portMappings,
        List<string> hostMappings, List<ReadinessCheckEntry> readinessChecks)
    {
        Identifier = identifier;
        Containerfile = containerfile;
        BuildContext = buildContext;
        ImageTag = imageTag;
        Network = network;
        EnvVars = envVars;
        _runtimeArgs = runtimeArgs;
        _portMappings = portMappings;
        HostMappings = hostMappings;
        _readinessChecks = readinessChecks;
    }

    public string ForFfi()
    {
        return ArenaJson.Serialize(new ContainerizedComponentConfig
        {
            Type = Type,
            Identifier = Identifier,
            Containerfile = Containerfile,
            BuildContext = BuildContext,
            ImageTag = ImageTag,
            Network = Network,
            EnvVars = EnvVars.ToDictionary(kv => kv.Key, kv => kv.Value),
            RuntimeArgs = RuntimeArgEntry.Build(_runtimeArgs),
            PortMappings = PortMappingEntry.Build(_portMappings),
            HostMappings = new List<string>(HostMappings),
            ReadinessChecks = ReadinessCheckWireFormat.Build(_readinessChecks),
        });
    }

    [JsonObject(ItemNullValueHandling = NullValueHandling.Ignore)]
    private sealed class ContainerizedComponentConfig
    {
        [JsonProperty("type")] public string Type { get; set; } = default!;
        [JsonProperty("identifier")] public string Identifier { get; set; } = default!;
        [JsonProperty("containerfile")] public string Containerfile { get; set; } = default!;
        [JsonProperty("build_context")] public string? BuildContext { get; set; }
        [JsonProperty("image_tag")] public string? ImageTag { get; set; }
        [JsonProperty("network")] public string? Network { get; set; }
        [JsonProperty("env_vars")] public Dictionary<string, string> EnvVars { get; set; } = default!;
        [JsonProperty("runtime_args")] public List<object> RuntimeArgs { get; set; } = default!;
        [JsonProperty("port_mappings")] public List<object> PortMappings { get; set; } = default!;
        [JsonProperty("host_mappings")] public List<string> HostMappings { get; set; } = default!;
        [JsonProperty("readiness_checks")] public List<object>? ReadinessChecks { get; set; }
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
    private readonly string _name;
    private string? _containerfile;
    private string? _buildContext;
    private string? _imageTag;
    private string? _network;
    private readonly Dictionary<string, string> _envVars = new();
    private readonly List<RuntimeArgEntry> _runtimeArgs = new();
    private readonly List<PortMappingEntry> _portMappings = new();
    private readonly List<string> _hostMappings = new();
    private readonly List<ReadinessCheckEntry> _readinessChecks = new();

    public ContainerizedComponentBuilder(string name)
    {
        _name = name;
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

    public ContainerizedComponent Build()
    {
        if (string.IsNullOrEmpty(_containerfile))
            throw new System.InvalidOperationException("containerfile must be set");
        var identifier = ArenaIdentifiers.Build("arena-container", _name);
        return new ContainerizedComponent(identifier, _containerfile, _buildContext, _imageTag, _network,
            new Dictionary<string, string>(_envVars), new List<RuntimeArgEntry>(_runtimeArgs),
            new List<PortMappingEntry>(_portMappings), new List<string>(_hostMappings),
            new List<ReadinessCheckEntry>(_readinessChecks));
    }
}
