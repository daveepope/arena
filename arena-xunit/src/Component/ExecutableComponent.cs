using System.Collections.Generic;
using System.Linq;
using ArenaDotnet.Xunit.Support;
using Newtonsoft.Json;

namespace ArenaDotnet.Xunit.Component;

public sealed class ExecutableComponent : IArenaMatchPiece
{
    public string Type => "exec";
    public string Identifier { get; }
    public string ExecutablePath { get; }
    public string? SourcePath { get; }
    public BuildTool? BuildTool { get; }
    public IReadOnlyDictionary<string, string> EnvVars { get; }

    private readonly List<RuntimeArgEntry> _runtimeArgs;
    private readonly List<ReadinessCheckEntry> _readinessChecks;

    internal ExecutableComponent(string identifier, string executablePath, string? sourcePath,
        BuildTool? buildTool, Dictionary<string, string> envVars, List<RuntimeArgEntry> runtimeArgs,
        List<ReadinessCheckEntry> readinessChecks)
    {
        Identifier = identifier;
        ExecutablePath = executablePath;
        SourcePath = sourcePath;
        BuildTool = buildTool;
        EnvVars = envVars;
        _runtimeArgs = runtimeArgs;
        _readinessChecks = readinessChecks;
    }

    public string ForFfi()
    {
        return ArenaJson.Serialize(new ExecutableComponentConfig
        {
            Type = Type,
            Identifier = Identifier,
            ExecutablePath = ExecutablePath,
            SourcePath = SourcePath,
            BuildTool = BuildTool?.ForFfi(),
            EnvVars = EnvVars.ToDictionary(kv => kv.Key, kv => kv.Value),
            RuntimeArgs = RuntimeArgEntry.Build(_runtimeArgs),
            ReadinessChecks = ReadinessCheckWireFormat.Build(_readinessChecks),
        });
    }

    [JsonObject(ItemNullValueHandling = NullValueHandling.Ignore)]
    private sealed class ExecutableComponentConfig
    {
        [JsonProperty("type")] public string Type { get; set; } = default!;
        [JsonProperty("identifier")] public string Identifier { get; set; } = default!;
        [JsonProperty("executable_path")] public string ExecutablePath { get; set; } = default!;
        [JsonProperty("source_path")] public string? SourcePath { get; set; }
        [JsonProperty("build_tool")] public object? BuildTool { get; set; }
        [JsonProperty("env_vars")] public Dictionary<string, string> EnvVars { get; set; } = default!;
        [JsonProperty("runtime_args")] public List<object> RuntimeArgs { get; set; } = default!;
        [JsonProperty("readiness_checks")] public List<object>? ReadinessChecks { get; set; }
    }
}

public sealed class ExecutableComponentBuilder
{
    private readonly string _name;
    private string? _executablePath;
    private string? _sourcePath;
    private BuildTool? _buildTool;
    private readonly Dictionary<string, string> _envVars = new();
    private readonly List<RuntimeArgEntry> _runtimeArgs = new();
    private readonly List<ReadinessCheckEntry> _readinessChecks = new();

    public ExecutableComponentBuilder(string name)
    {
        _name = name;
    }

    public ExecutableComponentBuilder WithExecutablePath(string path)
    {
        _executablePath = path;
        return this;
    }

    public ExecutableComponentBuilder WithSourcePath(string path)
    {
        _sourcePath = path;
        return this;
    }

    public ExecutableComponentBuilder WithBuildTool(BuildTool buildTool)
    {
        _buildTool = buildTool;
        return this;
    }

    public ExecutableComponentBuilder WithEnvVar(string key, string value)
    {
        _envVars[key] = value;
        return this;
    }

    public ExecutableComponentBuilder WithRuntimeArg(string name, string value)
    {
        _runtimeArgs.Add(new RuntimeArgEntry(name, value));
        return this;
    }

    public ExecutableComponentBuilder WithReadinessCheck(IArenaReadinessCheck check, string target)
    {
        return WithReadinessCheck(check, target, ReadinessCheckWireFormat.DefaultTimeoutMs);
    }

    public ExecutableComponentBuilder WithReadinessCheck(IArenaReadinessCheck check, string target, long timeoutMs)
    {
        _readinessChecks.Add(new ReadinessCheckEntry(check, target, timeoutMs));
        return this;
    }

    public ExecutableComponent Build()
    {
        if (string.IsNullOrEmpty(_executablePath))
            throw new System.InvalidOperationException("executable path must be set");
        var identifier = ArenaIdentifiers.Build("arena-exec", _name);
        return new ExecutableComponent(identifier, _executablePath, _sourcePath, _buildTool,
            new Dictionary<string, string>(_envVars), new List<RuntimeArgEntry>(_runtimeArgs),
            new List<ReadinessCheckEntry>(_readinessChecks));
    }
}
