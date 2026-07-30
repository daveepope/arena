using System.Collections.Generic;
using System.Linq;
using Newtonsoft.Json;

namespace ArenaXunit.Match;

public sealed class Match
{
    public string Name { get; }
    public IReadOnlyList<IArenaMatchPiece> Dependencies { get; }
    public IReadOnlyList<IArenaMatchPiece> Components { get; }
    public string? Network { get; }
    public IReadOnlyList<RegisteredPlaybook> Playbooks { get; }

    internal Match(string name, IReadOnlyList<IArenaMatchPiece> dependencies,
        IReadOnlyList<IArenaMatchPiece> components, string? network,
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
            Dependencies = Dependencies.Select(d => new { d.ForFfi() }).ToList(),
            Components = Components.Select(c => new { c.ForFfi() }).ToList(),
            Playbooks = Playbooks.Select(p => p.ToConfig()).ToList(),
        };
        return ArenaXunit.Support.ArenaJson.Serialize(obj);
    }

    [JsonObject(ItemNullValueHandling = NullValueHandling.Ignore)]
    private sealed class MatchConfig
    {
        [JsonProperty("match_name")] public string MatchName { get; set; } = default!;
        [JsonProperty("network")] public string? Network { get; set; }
        [JsonProperty("dependencies")] public List<object> Dependencies { get; set; } = default!;
        [JsonProperty("components")] public List<object> Components { get; set; } = default!;
        [JsonProperty("playbooks")] public List<object> Playbooks { get; set; } = default!;
    }
}
