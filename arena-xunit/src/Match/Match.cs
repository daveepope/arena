using System.Collections.Generic;
using System.Linq;
using ArenaDotnet.Xunit.Support;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;

namespace ArenaDotnet.Xunit;

public sealed class Match
{
    public string Name { get; }
    public IReadOnlyList<IArenaDependency> Dependencies { get; }
    public IReadOnlyList<IArenaComponent> Components { get; }
    public string? Network { get; }
    public IReadOnlyList<RegisteredPlaybook> Playbooks { get; }

    internal Match(string name, IReadOnlyList<IArenaDependency> dependencies,
        IReadOnlyList<IArenaComponent> components, string? network,
        IReadOnlyList<RegisteredPlaybook> playbooks)
    {
        Name = name;
        Dependencies = dependencies;
        Components = components;
        Network = network;
        Playbooks = playbooks;
    }

    public string ForFfi()
    {
        var obj = new MatchConfig
        {
            MatchName = Name,
            Network = Network,
            Dependencies = ChildrenWireFormat.Build(Dependencies) ?? new List<JToken>(),
            Components = ChildrenWireFormat.Build(Components) ?? new List<JToken>(),
            Playbooks = Playbooks.Select(p => p.ToConfig()).Where(c => c != null).ToList()!,
        };
        return ArenaDotnet.Xunit.Support.ArenaJson.Serialize(obj);
    }

    [JsonObject(ItemNullValueHandling = NullValueHandling.Ignore)]
    private sealed class MatchConfig
    {
        [JsonProperty("match_name")] public string MatchName { get; set; } = default!;
        [JsonProperty("network")] public string? Network { get; set; }
        [JsonProperty("dependencies")] public List<JToken> Dependencies { get; set; } = default!;
        [JsonProperty("components")] public List<JToken> Components { get; set; } = default!;
        [JsonProperty("playbooks")] public List<object> Playbooks { get; set; } = default!;
    }
}
