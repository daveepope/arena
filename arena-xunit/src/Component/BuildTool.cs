using System.Collections.Generic;
using Newtonsoft.Json;

namespace ArenaXunit.Component;

public sealed class BuildTool
{
    private readonly object _ffiValue;

    private BuildTool(object ffiValue)
    {
        _ffiValue = ffiValue;
    }

    public static BuildTool Cargo { get; } = new("cargo");
    public static BuildTool Maven { get; } = new("maven");
    public static BuildTool Gradle { get; } = new("gradle");
    public static BuildTool Dotnet { get; } = new("dotnet");
    public static BuildTool Make { get; } = new("make");
    public static BuildTool CMake { get; } = new("cmake");

    public static BuildTool Custom(string command, IEnumerable<string> args) =>
        new(new CustomBuildToolConfig { Command = command, Args = new List<string>(args) });

    internal object ForFfi() => _ffiValue;

    [JsonObject(ItemNullValueHandling = NullValueHandling.Ignore)]
    private sealed class CustomBuildToolConfig
    {
        [JsonProperty("command")] public string Command { get; set; } = default!;
        [JsonProperty("args")] public List<string> Args { get; set; } = default!;
    }
}
