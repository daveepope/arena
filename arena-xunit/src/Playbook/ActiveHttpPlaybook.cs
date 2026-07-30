using System;
using ArenaXunit.Ffi;
using ArenaXunit.Support;
using Newtonsoft.Json;

namespace ArenaXunit.Playbook;

public sealed class ActiveHttpPlaybook : ActivePlaybook
{
    public ActiveHttpPlaybook(IntPtr handle) : base(handle) { }

    public void Verify(string method, string path, int expectedCount)
    {
        var spec = ArenaJson.Serialize(new VerifySpec
        {
            Method = method,
            Path = path,
            ExpectedCount = expectedCount,
        });
        ArenaBindings.HttpPlaybookVerify(_handle, spec);
    }

    public void VerifyAtLeast(string method, string path, int minCount)
    {
        var spec = ArenaJson.Serialize(new VerifyAtLeastSpec
        {
            Method = method,
            Path = path,
            MinCount = minCount,
        });
        ArenaBindings.HttpPlaybookVerify(_handle, spec);
    }

    [JsonObject(ItemNullValueHandling = NullValueHandling.Ignore)]
    private sealed class VerifySpec
    {
        [JsonProperty("method")] public string? Method { get; set; }
        [JsonProperty("path")] public string? Path { get; set; }
        [JsonProperty("expected_count")] public int? ExpectedCount { get; set; }
    }

    [JsonObject(ItemNullValueHandling = NullValueHandling.Ignore)]
    private sealed class VerifyAtLeastSpec
    {
        [JsonProperty("method")] public string? Method { get; set; }
        [JsonProperty("path")] public string? Path { get; set; }
        [JsonProperty("min_count")] public int? MinCount { get; set; }
    }
}
